use crate::config::storage::sign;

use crate::db::database::CONN;
use crate::db::database::tables::rows::MarkingQueueRow;
use crate::db::database::tables::{papers as papers_db, question_bank};
use crate::pdf::{PaperPart, PaperPdfInput, PaperQuestion as PdfQuestion};
use crate::proto::services::question_bank::*;
use crate::types::error::{Error, OnConflict, Result};
use crate::types::paper::PaperStatus;
use crate::types::question::{
    AnswerSpaceType, BodyFormat, QuestionPart, QuestionUpdate, RubricCriterion,
};
use crate::types::token::Token;
use diesel::sql_query;
use diesel::sql_types::{Integer, Text};
use diesel::{Connection, OptionalExtension, RunQueryDsl};
use std::sync::Arc;
use tracing::{error, info};

pub struct QuestionBankService<C> {
    #[allow(dead_code)]
    config: Arc<C>,
}

// ---------------------------------------------------------------------------
// Helper: build a Question proto from a QuestionRow
// ---------------------------------------------------------------------------

fn build_question_proto(
    row: &crate::types::question::Question,
    rubric: &[RubricCriterion],
    parts: &[QuestionPart],
) -> Question {
    Question {
        id: row.id.unwrap_or(0),
        topic_id: row.topic,
        body: row.body.clone(),
        body_format: row.body_format as i32,
        stimulus: row.stimulus.clone(),
        r#type: row.type_ as i32,
        difficulty: row.difficulty as i32,
        cognitive_level: row.cognitive_level as i32,
        marks: row.marks as i32,
        max_marks: row.max_marks.map(|m| m as i32),
        answer_space_type: row.answer_space_type as i32,
        answer_lines: row.answer_lines.map(|l| l as i32),
        answer_box_height_mm: row.answer_box_height_mm.map(|h| h as i32),
        example_answer: row.example_answer.clone(),
        rubric: rubric.iter().map(|r| proto_rubric_criterion(r)).collect(),
        parts: parts.iter().map(|p| proto_question_part(p, &[])).collect(),
        created: row.created,
        updated: row.updated,
    }
}

fn proto_rubric_criterion(
    r: &RubricCriterion,
) -> crate::proto::services::question_bank::RubricCriterion {
    crate::proto::services::question_bank::RubricCriterion {
        position: r.position as i32,
        criterion: r.criterion.clone(),
        marks: r.marks as i32,
        max_marks: r.max_marks.map(|m| m as i32),
        required: r.required,
    }
}

fn proto_question_part(
    p: &QuestionPart,
    part_rubric: &[RubricCriterion],
) -> crate::proto::services::question_bank::QuestionPart {
    crate::proto::services::question_bank::QuestionPart {
        position: p.position as i32,
        label: p.label.clone(),
        body: p.body.clone(),
        body_format: p.body_format as i32,
        marks: p.marks as i32,
        max_marks: p.max_marks.map(|m| m as i32),
        answer_space_type: p.answer_space_type as i32,
        answer_lines: p.answer_lines.map(|l| l as i32),
        answer_box_height_mm: p.answer_box_height_mm.map(|h| h as i32),
        example_answer: p.example_answer.clone(),
        stimulus: p.stimulus.clone(),
        rubric: part_rubric
            .iter()
            .map(|r| proto_rubric_criterion(r))
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Helper: load a full Question (row + rubric + parts + images) inside CONN
// ---------------------------------------------------------------------------

fn load_full_question(conn: &mut diesel::SqliteConnection, id: i32) -> Result<Question> {
    let row = question_bank::get_question(conn, id)?.ok_or(Error::NotFound)?;
    let rubric = question_bank::get_rubric_criteria(conn, id)?;
    let parts = question_bank::get_question_parts(conn, id)?;
    Ok(build_question_proto(&row, &rubric, &parts))
}

// QuestionRow is kept for list_questions which returns typed Question directly

// ---------------------------------------------------------------------------
// Helper: map RubricCriterionInput → DB tuple
// ---------------------------------------------------------------------------

fn rubric_input_tuples(
    inputs: &[RubricCriterionInput],
) -> Vec<(i16, String, i16, Option<i16>, bool)> {
    inputs
        .iter()
        .enumerate()
        .map(|(i, r)| {
            (
                (i + 1) as i16,
                r.criterion.clone(),
                r.marks as i16,
                r.max_marks.map(|m| m as i16),
                r.required,
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Helper: validate rubric marks against question marks
// ---------------------------------------------------------------------------

fn validate_rubric_marks(question_marks: i16, rubric: &[RubricCriterionInput]) -> Result<()> {
    if rubric.is_empty() {
        return Ok(());
    }
    let rubric_sum: i32 = rubric.iter().map(|r| r.marks).sum();
    if rubric_sum < question_marks as i32 {
        return Err(Error::InvalidRubricMarks);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Helper: map MarkingQueueRow → proto
// ---------------------------------------------------------------------------

fn marking_row_to_response(row: &MarkingQueueRow) -> MarkingStatusResponse {
    MarkingStatusResponse {
        phase: row.phase as i32,
        progress: row.progress.clone(),
        error: row.error.clone(),
        total_students: row.total_students,
        marked_students: row.marked_students,
    }
}

// ---------------------------------------------------------------------------
// Helper row types
// ---------------------------------------------------------------------------

#[derive(diesel::QueryableByName)]
struct SchoolInfoRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub motto: Option<String>,
}

#[derive(diesel::QueryableByName)]
struct SubjectNameRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub name: String,
}

/// Resolve a topic by natural key, auto-creating the subject and/or topic
/// if they don't exist yet on the server. Returns the server-side topic ID.
fn resolve_topic(
    conn: &mut diesel::SqliteConnection,
    subject_name: &str,
    curriculum: i32,
    grade: i32,
    topic_name: &str,
) -> Result<i32> {
    use diesel::sql_types::{BigInt, Integer, SmallInt, Text};

    #[derive(diesel::QueryableByName)]
    struct IdRow {
        #[diesel(sql_type = Integer)]
        id: i32,
    }

    let now = chrono::Utc::now().timestamp();

    // 1. Look up or auto-create the subject
    let subject_id: i32 = match sql_query(
        "SELECT id FROM subjects WHERE name = ? AND curriculum = ? LIMIT 1",
    )
    .bind::<Text, _>(subject_name)
    .bind::<SmallInt, _>(curriculum as i16)
    .get_result::<IdRow>(conn)
    .optional()
    .map_err(|e| Error::internal(e))?
    {
        Some(row) => row.id,
        None => {
            sql_query(
                "INSERT INTO subjects (name, curriculum, created, updated) VALUES (?, ?, ?, ?)",
            )
            .bind::<Text, _>(subject_name)
            .bind::<SmallInt, _>(curriculum as i16)
            .bind::<BigInt, _>(now)
            .bind::<BigInt, _>(now)
            .execute(conn)
            .map_err(|e| Error::internal(e))?;

            sql_query(
                "SELECT id FROM subjects WHERE name = ? AND curriculum = ? LIMIT 1",
            )
            .bind::<Text, _>(subject_name)
            .bind::<SmallInt, _>(curriculum as i16)
            .get_result::<IdRow>(conn)
            .map(|r| r.id)
            .map_err(|e| {
                Error::internal(format!("subject not found after auto-create: {e}"))
            })?
        }
    };

    // 2. Look up or auto-create the topic
    let topic_id: i32 = match sql_query(
        "SELECT id FROM topics WHERE subject = ? AND grade = ? AND name = ? LIMIT 1",
    )
    .bind::<Integer, _>(subject_id)
    .bind::<SmallInt, _>(grade as i16)
    .bind::<Text, _>(topic_name)
    .get_result::<IdRow>(conn)
    .optional()
    .map_err(|e| Error::internal(e))?
    {
        Some(row) => row.id,
        None => {
            sql_query(
                "INSERT INTO topics (subject, grade, name, created, updated) VALUES (?, ?, ?, ?, ?)",
            )
            .bind::<Integer, _>(subject_id)
            .bind::<SmallInt, _>(grade as i16)
            .bind::<Text, _>(topic_name)
            .bind::<BigInt, _>(now)
            .bind::<BigInt, _>(now)
            .execute(conn)
            .map_err(|e| Error::internal(e))?;

            sql_query(
                "SELECT id FROM topics WHERE subject = ? AND grade = ? AND name = ? LIMIT 1",
            )
            .bind::<Integer, _>(subject_id)
            .bind::<SmallInt, _>(grade as i16)
            .bind::<Text, _>(topic_name)
            .get_result::<IdRow>(conn)
            .map(|r| r.id)
            .map_err(|e| {
                Error::internal(format!("topic not found after auto-create: {e}"))
            })?
        }
    };

    Ok(topic_id)
}

// =========================================================================
// QuestionBank trait implementation
// =========================================================================

impl<C: Send + Sync + 'static> QuestionBank for QuestionBankService<C> {
    type Config = Arc<C>;

    fn new(config: Self::Config) -> QuestionBankServer<Self> {
        QuestionBankServer::new(Self { config })
    }

    // ── create_question ──────────────────────────────────────────────────

    async fn create_question(
        &self,
        token: Token,
        req: CreateQuestionRequest,
    ) -> Result<CreateQuestionResponse> {
        let user_id = token.user.to_string();

        // A question with a stimulus (passage/poem/narrative) is valid even without body.
        if req.body.trim().is_empty()
            && req
                .stimulus
                .as_ref()
                .map_or(true, |s| s.trim().is_empty())
        {
            return Err(Error::InvalidQuestionText);
        }
        if req.marks <= 0 {
            return Err(Error::InvalidQuestionMarks);
        }

        let question = CONN.with(|conn| {
            conn.transaction(|conn| {
                let qid = question_bank::insert_question(
                    conn,
                    req.topic_id,
                    &req.body,
                    (req.body_format as i16)
                        .try_into()
                        .unwrap_or(BodyFormat::Plain),
                    req.stimulus.as_deref(),
                    (req.r#type as i16).try_into().unwrap_or_default(),
                    req.difficulty as i16,
                    (req.cognitive_level as i16).try_into().unwrap_or_default(),
                    req.marks as i16,
                    req.max_marks.map(|m| m as i16),
                    (req.answer_space_type as i16)
                        .try_into()
                        .unwrap_or(AnswerSpaceType::Lines),
                    req.answer_lines.map(|l| l as i16),
                    req.answer_box_height_mm.map(|h| h as i16),
                    req.example_answer.as_deref(),
                    &user_id,
                )
                .on_conflict(Error::QuestionAlreadyExists)?;

                if !req.rubric.is_empty() {
                    validate_rubric_marks(req.marks as i16, &req.rubric)?;
                    let tuples = rubric_input_tuples(&req.rubric);
                    question_bank::insert_rubric_criteria(conn, qid, &tuples)?;
                }

                if !req.parts.is_empty() {
                    let parts: Vec<QuestionPart> = req
                        .parts
                        .iter()
                        .enumerate()
                        .map(|(i, p)| QuestionPart {
                            question: qid,
                            position: (i + 1) as i16,
                            label: p.label.clone(),
                            body: p.body.clone(),
                            body_format: (p.body_format as i16)
                                .try_into()
                                .unwrap_or(BodyFormat::Plain),
                            marks: p.marks as i16,
                            max_marks: p.max_marks.map(|m| m as i16),
                            answer_space_type: (p.answer_space_type as i16)
                                .try_into()
                                .unwrap_or(AnswerSpaceType::Lines),
                            answer_lines: p.answer_lines.map(|l| l as i16),
                            answer_box_height_mm: p.answer_box_height_mm.map(|h| h as i16),
                            example_answer: p.example_answer.clone(),
                            stimulus: p.stimulus.clone(),
                        })
                        .collect();
                    question_bank::insert_question_parts(conn, qid, &parts)?;

                    // Insert part rubric criteria
                    for (part_idx, p) in req.parts.iter().enumerate() {
                        if !p.rubric.is_empty() {
                            validate_rubric_marks(p.marks as i16, &p.rubric)?;
                            let part_tuples: Vec<(i16, String, i16, Option<i16>, bool)> = p.rubric.iter().enumerate().map(|(ri, r)| {
                                ((ri + 1) as i16, r.criterion.clone(), r.marks as i16, r.max_marks.map(|m| m as i16), r.required)
                            }).collect();
                            let _ = question_bank::insert_part_rubric_criteria(conn, qid, (part_idx + 1) as i16, &part_tuples);
                        }
                    }
                }

                load_full_question(conn, qid)
            })
        })?;

        Ok(CreateQuestionResponse {
            question: Some(question),
        })
    }

    // ── update_question ──────────────────────────────────────────────────

    async fn update_question(
        &self,
        _token: Token,
        req: UpdateQuestionRequest,
    ) -> Result<UpdateQuestionResponse> {
        let qid = req.question_id;

        let changeset = QuestionUpdate {
            topic: req.topic_id,
            body: req.body.clone(),
            body_format: req.body_format.and_then(|v| (v as i16).try_into().ok()),
            stimulus: req.stimulus.map(Some),
            type_: req.r#type.and_then(|v| (v as i16).try_into().ok()),
            difficulty: req.difficulty.map(|v| v as i16),
            cognitive_level: req.cognitive_level.and_then(|v| (v as i16).try_into().ok()),
            marks: req.marks.map(|v| v as i16),
            max_marks: req.max_marks.map(|v| Some(v as i16)),
            answer_space_type: req
                .answer_space_type
                .and_then(|v| (v as i16).try_into().ok()),
            answer_lines: req.answer_lines.map(|v| Some(v as i16)),
            answer_box_height_mm: req.answer_box_height_mm.map(|v| Some(v as i16)),
            example_answer: req.example_answer.map(Some),
            updated: Some(chrono::Utc::now().timestamp()),
        };

        let question = CONN.with(|conn| {
            conn.transaction(|conn| {
                question_bank::update_question(conn, qid, changeset)?;

                if !req.rubric.is_empty() {
                    let tuples = rubric_input_tuples(&req.rubric);
                    question_bank::replace_rubric_criteria(conn, qid, &tuples)?;
                }

                load_full_question(conn, qid)
            })
        })?;

        Ok(UpdateQuestionResponse {
            question: Some(question),
        })
    }

    // ── delete_question ──────────────────────────────────────────────────

    async fn delete_question(
        &self,
        _token: Token,
        req: DeleteQuestionRequest,
    ) -> Result<DeleteQuestionResponse> {
        CONN.with(|conn| {
            question_bank::delete_question(conn, req.question_id)
        })?;
        Ok(DeleteQuestionResponse {})
    }

    // ── bulk_import ──────────────────────────────────────────────────────

    async fn bulk_import(
        &self,
        token: Token,
        req: BulkImportRequest,
    ) -> Result<BulkImportResponse> {
        let user_id = token.user.to_string();
        let mut created_count: i32 = 0;
        let mut duplicates_skipped: i32 = 0;
        let mut errors: Vec<String> = Vec::new();

        let result = CONN.with(|conn| {
            conn.transaction(|conn| {
                // Resolve topic by natural key when provided; otherwise
                // fall back to per-question topic_id for backwards compat.
                let resolved_topic_id: Option<i32> =
                    if !req.subject_name.is_empty() {
                        Some(resolve_topic(
                            conn,
                            &req.subject_name,
                            req.curriculum,
                            req.grade,
                            &req.topic_name,
                        )?)
                    } else {
                        None
                    };

                for (idx, q) in req.questions.iter().enumerate() {
                    let topic_id = resolved_topic_id.unwrap_or(q.topic_id);
                    // A question with a stimulus (passage/poem/narrative) is valid even without body.
                    let body_empty = q.body.trim().is_empty();
                    let stimulus_empty = q
                        .stimulus
                        .as_ref()
                        .map_or(true, |s| s.trim().is_empty());
                    if (body_empty && stimulus_empty) || q.marks <= 0 {
                        duplicates_skipped += 1;
                        errors.push(format!(
                            "Q{}: empty body or invalid marks (marks={})",
                            idx + 1,
                            q.marks
                        ));
                        continue;
                    }

                    match question_bank::find_or_insert_question(
                        conn,
                        topic_id,
                        &q.body,
                        (q.body_format as i16)
                            .try_into()
                            .unwrap_or(BodyFormat::Plain),
                        q.stimulus.as_deref(),
                        (q.r#type as i16).try_into().unwrap_or_default(),
                        q.difficulty as i16,
                        (q.cognitive_level as i16).try_into().unwrap_or_default(),
                        q.marks as i16,
                        q.max_marks.map(|m| m as i16),
                        (q.answer_space_type as i16)
                            .try_into()
                            .unwrap_or(AnswerSpaceType::Lines),
                        q.answer_lines.map(|l| l as i16),
                        q.answer_box_height_mm.map(|h| h as i16),
                        q.example_answer.as_deref(),
                        &user_id,
                    ) {
                        Ok((qid, is_new)) => {
                            if !is_new {
                                duplicates_skipped += 1;
                                continue;
                            }
                            if !q.rubric.is_empty() {
                                if let Err(e) = validate_rubric_marks(q.marks as i16, &q.rubric) {
                                    let msg = format!("Q{}: {e}", idx + 1);
                                    error!("bulk_import: {msg}");
                                    errors.push(msg);
                                    duplicates_skipped += 1;
                                    continue;
                                }
                                let tuples = rubric_input_tuples(&q.rubric);
                                let _ = question_bank::insert_rubric_criteria(conn, qid, &tuples);
                            }

                            // Insert question parts and their rubric criteria
                            if !q.parts.is_empty() {
                                let parts: Vec<QuestionPart> = q.parts.iter().enumerate().map(|(i, p)| {
                                    QuestionPart {
                                        question: qid,
                                        position: (i + 1) as i16,
                                        label: p.label.clone(),
                                        body: p.body.clone(),
                                        body_format: (p.body_format as i16).try_into().unwrap_or(BodyFormat::Plain),
                                        marks: p.marks as i16,
                                        max_marks: p.max_marks.map(|m| m as i16),
                                        answer_space_type: (p.answer_space_type as i16).try_into().unwrap_or(AnswerSpaceType::Lines),
                                        answer_lines: p.answer_lines.map(|l| l as i16),
                                        answer_box_height_mm: p.answer_box_height_mm.map(|h| h as i16),
                                        example_answer: p.example_answer.clone(),
                                        stimulus: p.stimulus.clone(),
                                    }
                                }).collect();

                                if let Err(e) = question_bank::insert_question_parts(conn, qid, &parts) {
                                    let msg = format!(
                                        "Q{}: part insertion failed: {e}",
                                        idx + 1
                                    );
                                    error!("bulk_import: {msg}");
                                    errors.push(msg);
                                    duplicates_skipped += 1;
                                    continue;
                                }

                                // Insert part rubric criteria
                                for (part_idx, p) in q.parts.iter().enumerate() {
                                    if !p.rubric.is_empty() {
                                        if let Err(e) = validate_rubric_marks(p.marks as i16, &p.rubric) {
                                            let msg = format!("Q{} part {}: {e}", idx + 1, part_idx + 1);
                                            error!("bulk_import: {msg}");
                                            errors.push(msg);
                                            continue;
                                        }
                                        let part_tuples: Vec<(i16, String, i16, Option<i16>, bool)> = p.rubric.iter().enumerate().map(|(ri, r)| {
                                            ((ri + 1) as i16, r.criterion.clone(), r.marks as i16, r.max_marks.map(|m| m as i16), r.required)
                                        }).collect();
                                        let _ = question_bank::insert_part_rubric_criteria(conn, qid, (part_idx + 1) as i16, &part_tuples);
                                    }
                                }
                            }
                            created_count += 1;
                        }
                        Err(e) => {
                            let msg = format!(
                                "Q{}: {e}",
                                idx + 1
                            );
                            error!(
                                "bulk_import: question insert failed topic={} body={}: {e}",
                                topic_id,
                                &q.body
                            );
                            errors.push(msg);
                            duplicates_skipped += 1;
                        }
                    }
                }

                Ok::<_, Error>(BulkImportResponse {
                    created: created_count,
                    skipped: duplicates_skipped,
                    errors,
                })
            })
        })?;

        Ok(result)
    }

    // ── request_image_upload_urls ─────────────────────────────────────────

    async fn request_image_upload_urls(
        &self,
        _token: Token,
        req: ImageUploadUrlsRequest,
    ) -> Result<ImageUploadUrlsResponse> {
        let urls = CONN.with(|conn| {
            let mut result = Vec::with_capacity(req.count as usize);

            for i in 0..req.count {
                let key = format!("questions/{}/image_{}.webp", req.question_id, i + 1);
                let put_url = sign::url(&key, sign::PUT_TTL, true);

                question_bank::insert_question_image(
                    conn,
                    req.question_id,
                    (i + 1) as i16,
                    0,
                    &key,
                    None,
                )?;

                result.push(put_url);
            }

            Ok(result) as Result<Vec<String>>
        })?;

        Ok(ImageUploadUrlsResponse { urls })
    }

    // ── generate_paper ───────────────────────────────────────────────────

    async fn generate_paper(
        &self,
        _token: Token,
        req: GeneratePaperRequest,
    ) -> Result<GeneratePaperResponse> {
        let result = CONN.with(|conn| {

            // Load paper
            let paper = papers_db::get_paper(conn, &req.paper_id)?.ok_or(Error::PaperNotFound)?;

            info!(
                "generate_paper: paper_id={} paper_total_marks={} n_allocations={}",
                req.paper_id,
                paper.total_marks,
                req.topic_allocations.len(),
            );

            let alloc_sum: i32 = req.topic_allocations.iter().map(|ta| ta.total_marks).sum();
            if alloc_sum != paper.total_marks as i32 {
                error!(
                    "generate_paper: alloc_sum ({}) != paper.total_marks ({}) — check failed",
                    alloc_sum, paper.total_marks,
                );
                return Err(Error::NotEnoughQuestionsForAllocation);
            }

            // Clear existing class-wide questions
            question_bank::delete_paper_questions(conn, &req.paper_id, None)?;

            let mut all_questions: Vec<(i32, i16)> = Vec::new();
            let mut position: i16 = 0;

            for alloc in &req.topic_allocations {
                let selected = question_bank::select_questions_for_paper(
                    conn,
                    alloc.topic_id,
                    &[],
                    None,
                )?;

                if selected.is_empty() {
                    error!(
                        "generate_paper: no questions found for topic_id={} alloc_marks={}",
                        alloc.topic_id, alloc.total_marks,
                    );
                    return Err(Error::NotEnoughQuestionsForAllocation);
                }

                info!(
                    "generate_paper: topic_id={} alloc_marks={} candidates={}",
                    alloc.topic_id,
                    alloc.total_marks,
                    selected.len(),
                );

                let mut remaining = alloc.total_marks as i16;
                for q in &selected {
                    if remaining <= 0 {
                        break;
                    }
                    all_questions.push((q.id.unwrap_or(0), position));
                    position += 1;
                    remaining -= q.marks;
                }
            }

            question_bank::insert_paper_questions(conn, &req.paper_id, None, &all_questions)?;

            // Transition to QuestionsSet
            let _ =
                papers_db::transition_paper_status(conn, &req.paper_id, PaperStatus::QuestionsSet);

            Ok::<_, Error>(GeneratePaperResponse {
                success: true,
                message: format!("Generated {} questions", all_questions.len()),
            })
        })?;

        Ok(result)
    }

    // ── get_paper_questions ──────────────────────────────────────────────

    async fn get_paper_questions(
        &self,
        token: Token,
        req: GetPaperQuestionsRequest,
    ) -> Result<GetPaperQuestionsResponse> {
        let questions = CONN.with(|conn| {

            // Check paper status
            let paper = papers_db::get_paper(conn, &req.paper_id)?.ok_or(Error::PaperNotFound)?;

            let user_id = token.user.to_string();
            let is_teacher = paper.teacher == user_id;

            // Teachers can see from Finalized(2) onward; others only from Revealed(3)
            let min_status = if is_teacher {
                PaperStatus::Finalized
            } else {
                PaperStatus::Revealed
            };

            if paper.status < min_status {
                return Err(Error::PaperNotRevealed);
            }

            let rows = question_bank::get_paper_questions(conn, &req.paper_id, req.student)?;

            let mut result = Vec::with_capacity(rows.len());
            for row in &rows {
                let q = load_full_question(conn, row.question)?;
                result.push(q);
            }

            Ok::<_, Error>(result)
        })?;

        Ok(GetPaperQuestionsResponse { questions })
    }

    // ── regenerate_question ──────────────────────────────────────────────

    async fn regenerate_question(
        &self,
        _token: Token,
        req: RegenerateQuestionRequest,
    ) -> Result<RegenerateQuestionResponse> {
        let question = CONN.with(|conn| {

            let current_pqs = question_bank::get_paper_questions(conn, &req.paper_id, req.student)?;

            let mut exclude: Vec<i32> = current_pqs.iter().map(|pq| pq.question).collect();
            exclude.extend_from_slice(&req.exclude_ids);

            let candidates = question_bank::select_questions_for_paper(
                conn,
                req.topic_id,
                &exclude,
                None,
            )?;

            let replacement = candidates
                .first()
                .ok_or(Error::NotEnoughQuestionsForAllocation)?;
            let new_id = replacement.id.unwrap_or(0);

            // Swap at the given position
            let paper_qs_updated: Vec<(i32, i16)> = current_pqs
                .iter()
                .map(|pq| {
                    if pq.position == req.position as i16 {
                        (new_id, pq.position)
                    } else {
                        (pq.question, pq.position)
                    }
                })
                .collect();

            question_bank::delete_paper_questions(conn, &req.paper_id, req.student)?;
            question_bank::insert_paper_questions(
                conn,
                &req.paper_id,
                req.student,
                &paper_qs_updated,
            )?;

            load_full_question(conn, new_id)
        })?;

        Ok(RegenerateQuestionResponse {
            question: Some(question),
        })
    }

    // ── clear_paper_questions ────────────────────────────────────────────

    async fn clear_paper_questions(
        &self,
        _token: Token,
        req: ClearPaperQuestionsRequest,
    ) -> Result<ClearPaperQuestionsResponse> {
        CONN.with(|conn| {
            question_bank::delete_paper_questions(conn, &req.paper_id, req.student)
        })?;

        Ok(ClearPaperQuestionsResponse {})
    }

    // ── finalize_paper ───────────────────────────────────────────────────

    async fn finalize_paper(
        &self,
        _token: Token,
        req: FinalizePaperRequest,
    ) -> Result<FinalizePaperResponse> {
        // ── Phase 1: load all data synchronously ─────────────────────────
        struct PaperData {
            school_name: String,
            school_motto: Option<String>,
            paper_name: String,
            subject_name: String,
            grade: i16,
            duration_minutes: i16,
            instructions: Option<String>,
            questions: Vec<PdfQuestion>,
            pdf_key: String,
            ms_key: String,
        }

        let data = CONN.with(|conn| {

            let paper = papers_db::get_paper(conn, &req.paper_id)?.ok_or(Error::PaperNotFound)?;

            let paper_qs = question_bank::get_paper_questions(conn, &req.paper_id, None)?;
            if paper_qs.is_empty() {
                return Err(Error::NotEnoughQuestionsForAllocation);
            }

            // Load school info
            let school_info: Option<SchoolInfoRow> =
                sql_query("SELECT name, motto FROM schools WHERE id = ?")
                    .bind::<Text, _>(&paper.school)
                    .get_result(conn)
                    .optional()
                    .map_err(Error::internal)?;
            let school_name = school_info
                .as_ref()
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "School".into());
            let school_motto = school_info.as_ref().and_then(|s| s.motto.clone());

            // Load subject name
            let subject_info: Option<SubjectNameRow> =
                sql_query("SELECT name FROM subjects WHERE id = ?")
                    .bind::<Integer, _>(paper.subject)
                    .get_result(conn)
                    .optional()
                    .map_err(Error::internal)?;
            let subject_name = subject_info
                .as_ref()
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "Subject".into());

            // Build question data
            let mut pdf_questions: Vec<PdfQuestion> = Vec::with_capacity(paper_qs.len());

            for pq in &paper_qs {
                let row = question_bank::get_question(conn, pq.question)?.ok_or(Error::NotFound)?;
                let rubric = question_bank::get_rubric_criteria(conn, pq.question)?;
                let parts = question_bank::get_question_parts(conn, pq.question)?;

                let pdf_parts: Vec<PaperPart> = parts
                    .iter()
                    .map(|p| PaperPart {
                        label: p.label.clone(),
                        body: p.body.clone(),
                        body_format: p.body_format as u8,
                        marks: p.marks,
                        answer_space_type: p.answer_space_type as u8,
                        answer_lines: p.answer_lines,
                        answer_box_height_mm: p.answer_box_height_mm,
                        stimulus: p.stimulus.clone(),
                        rubric: Vec::new(),
                    })
                    .collect();

                let rubric_data: Vec<(String, i16, bool)> = rubric
                    .iter()
                    .map(|r| (r.criterion.clone(), r.marks, r.required))
                    .collect();

                pdf_questions.push(PdfQuestion {
                    body: row.body.clone(),
                    body_format: row.body_format as u8,
                    marks: row.marks,
                    max_marks: row.max_marks,
                    answer_space_type: row.answer_space_type as u8,
                    answer_lines: row.answer_lines,
                    answer_box_height_mm: row.answer_box_height_mm,
                    stimulus: row.stimulus.clone(),
                    example_answer: row.example_answer.clone(),
                    rubric: rubric_data,
                    parts: pdf_parts,
                    section: pq.section.clone(),
                });
            }

            Ok::<_, Error>(PaperData {
                pdf_key: format!("papers/{}/paper.pdf", req.paper_id),
                ms_key: format!("papers/{}/marking_scheme.pdf", req.paper_id),
                paper_name: paper.name.clone(),
                school_name,
                school_motto,
                subject_name,
                grade: paper.grade,
                duration_minutes: paper.duration_minutes,
                instructions: paper.instructions.clone(),
                questions: pdf_questions,
            })
        })?;

        let pdf_key = data.pdf_key;
        let ms_key = data.ms_key;

        // ── Phase 2: generate PDFs ────────────────────────────────────────

        let pdf_input = PaperPdfInput {
            school_name: &data.school_name,
            school_motto: data.school_motto.as_deref(),
            paper_name: &data.paper_name,
            subject_name: &data.subject_name,
            paper_number: None,
            grade: data.grade,
            duration_minutes: Some(data.duration_minutes),
            instructions: data.instructions.as_deref(),
            questions: &data.questions,
        };

        let pdf_bytes = crate::pdf::generate_paper_pdf_typst(&pdf_input).map_err(|e| {
            error!("PDF generation failed: {}", e);
            Error::internal(e)
        })?;

        let scheme_bytes =
            crate::pdf::generate_marking_scheme_pdf_typst(&pdf_input).map_err(|e| {
                error!("Marking scheme generation failed: {}", e);
                Error::internal(e)
            })?;

        // ── Phase 3: upload to R2 ────────────────────────────────────────

        let client = reqwest::Client::new();

        let put_url = sign::presign(
            env!("R2_ACCOUNT_ID"),
            env!("R2_BUCKET"),
            env!("R2_ACCESS_KEY_ID"),
            env!("R2_SECRET_ACCESS_KEY"),
            "PUT",
            &pdf_key,
            sign::PUT_TTL,
            Some("application/pdf"),
        );

        let resp = client
            .put(&put_url)
            .header("Content-Type", "application/pdf")
            .body(pdf_bytes)
            .send()
            .await
            .map_err(Error::internal)?;

        if !resp.status().is_success() {
            let msg = format!("R2 PDF upload failed: HTTP {}", resp.status());
            error!("{msg}");
            return Err(Error::Internal(msg));
        }

        let ms_put_url = sign::presign(
            env!("R2_ACCOUNT_ID"),
            env!("R2_BUCKET"),
            env!("R2_ACCESS_KEY_ID"),
            env!("R2_SECRET_ACCESS_KEY"),
            "PUT",
            &ms_key,
            sign::PUT_TTL,
            Some("application/pdf"),
        );

        let ms_resp = client
            .put(&ms_put_url)
            .header("Content-Type", "application/pdf")
            .body(scheme_bytes)
            .send()
            .await
            .map_err(Error::internal)?;

        if !ms_resp.status().is_success() {
            let msg = format!("R2 marking scheme upload failed: HTTP {}", ms_resp.status());
            error!("{msg}");
            return Err(Error::Internal(msg));
        }

        // ── Phase 4: persist keys + transition status ────────────────────

        CONN.with(|conn| {
            let update = crate::types::paper::PaperUpdate {
                pdf_key: Some(Some(pdf_key.clone())),
                ms_key: Some(Some(ms_key.clone())),
                updated: Some(chrono::Utc::now().timestamp()),
                ..Default::default()
            };
            papers_db::update_paper(conn, &req.paper_id, update)?;
            papers_db::transition_paper_status(conn, &req.paper_id, PaperStatus::Finalized)?;
            Ok::<_, Error>(())
        })?;

        Ok(FinalizePaperResponse { pdf_key, ms_key })
    }

    // ── list_questions ───────────────────────────────────────────────────

    async fn list_questions(
        &self,
        _token: Token,
        req: ListQuestionsRequest,
    ) -> Result<ListQuestionsResponse> {
        let page = req.page.max(0) as i64;
        let page_size = if req.page_size <= 0 {
            50
        } else {
            req.page_size as i64
        };

        let (questions, total) = CONN.with(|conn| {

            let total = question_bank::count_questions(conn, req.topic_id)?;

            let rows =
                question_bank::list_questions(conn, req.topic_id, page * page_size, page_size)?;

            let mut questions = Vec::with_capacity(rows.len());
            for row in &rows {
                let id = row.id.unwrap_or(0);
                let rubric = question_bank::get_rubric_criteria(conn, id)?;
                let parts = question_bank::get_question_parts(conn, id)?;
                questions.push(build_question_proto(&row, &rubric, &parts));
            }

            Ok::<_, Error>((questions, total))
        })?;

        Ok(ListQuestionsResponse { questions, total })
    }

    // ── get_question ─────────────────────────────────────────────────────

    async fn get_question(
        &self,
        _token: Token,
        req: GetQuestionRequest,
    ) -> Result<GetQuestionResponse> {
        let question = CONN.with(|conn| {
            load_full_question(conn, req.question_id)
        })?;

        Ok(GetQuestionResponse {
            question: Some(question),
        })
    }

    // ── get_question_grades ──────────────────────────────────────────────

    async fn get_question_grades(
        &self,
        _token: Token,
        req: GetQuestionGradesRequest,
    ) -> Result<GetQuestionGradesResponse> {
        let grades = CONN.with(|conn| {

            let grade_rows =
                question_bank::get_question_grades_for_student(conn, &req.paper_id, req.student)?;

            let result: Vec<QuestionGrade> = grade_rows
                .iter()
                .map(|gr| QuestionGrade {
                    question_id: gr.question,
                    score: gr.score,
                    feedback: gr.feedback.clone(),
                })
                .collect();

            Ok::<_, Error>(result)
        })?;

        Ok(GetQuestionGradesResponse { grades })
    }

    // ── get_marking_status ───────────────────────────────────────────────

    async fn get_marking_status(
        &self,
        _token: Token,
        req: MarkingStatusRequest,
    ) -> Result<MarkingStatusResponse> {
        let row = CONN.with(|conn| {
            question_bank::get_marking_status(conn, &req.paper_id)
        })?;

        match row {
            Some(r) => Ok(marking_row_to_response(&r)),
            None => Err(Error::PaperNotFound),
        }
    }
}
