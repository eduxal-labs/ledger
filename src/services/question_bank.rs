use crate::config::storage::sign;
use crate::db::database::CONN;
use crate::db::database::tables::question_bank;
use crate::db::database::tables::rows::{
    MarkingQueueRow, QuestionImageRow, QuestionRow, RubricCriterionRow,
};
use crate::proto::services::question_bank::*;
use crate::types::error::{Error, Result};
use crate::types::token::Token;
use diesel::sql_query;
use diesel::sql_types::{Integer, SmallInt, Text};
use diesel::{Connection, RunQueryDsl};
use std::sync::Arc;
use tracing::error;

pub struct QuestionBankService<C> {
    #[allow(dead_code)]
    config: Arc<C>,
}

// ---------------------------------------------------------------------------
// Helper: build a Question proto from DB rows
// ---------------------------------------------------------------------------

fn build_question_proto(
    row: &QuestionRow,
    rubric: &[RubricCriterionRow],
    images: &[QuestionImageRow],
    sign_urls: bool,
) -> Question {
    Question {
        id: row.id,
        topic_id: row.topic,
        text: row.text.clone(),
        marks: row.marks as i32,
        example_answer: row.example_answer.clone(),
        rubric: rubric
            .iter()
            .map(|r| RubricCriterion {
                position: r.position as i32,
                criterion: r.criterion.clone(),
                marks: r.marks as i32,
            })
            .collect(),
        images: images
            .iter()
            .map(|img| {
                let url = if sign_urls {
                    Some(sign::url(&img.key, sign::GET_TTL, false))
                } else {
                    None
                };
                QuestionImage {
                    id: img.id,
                    position: img.position as i32,
                    context: img.context as i32,
                    key: img.key.clone(),
                    url,
                    caption: img.caption.clone(),
                }
            })
            .collect(),
        created: row.created,
        updated: row.updated,
    }
}

// ---------------------------------------------------------------------------
// Helper: load a full Question (row + rubric + images) inside a CONN closure
// ---------------------------------------------------------------------------

fn load_full_question(
    conn: &mut diesel::SqliteConnection,
    id: i32,
    sign_urls: bool,
) -> Result<Question> {
    let row = question_bank::get_question(conn, id)?;
    let rubric = question_bank::get_rubric_criteria(conn, id)?;
    let images = question_bank::get_question_images(conn, id)?;
    Ok(build_question_proto(&row, &rubric, &images, sign_urls))
}

// ---------------------------------------------------------------------------
// Helper: convert RubricCriterionInput vec to the tuple format the DB expects
// ---------------------------------------------------------------------------

fn rubric_tuples(inputs: &[RubricCriterionInput]) -> Vec<(i16, String, i16)> {
    inputs
        .iter()
        .enumerate()
        .map(|(i, r)| ((i + 1) as i16, r.criterion.clone(), r.marks as i16))
        .collect()
}

// ---------------------------------------------------------------------------
// Helper: map a MarkingQueueRow to MarkingStatusResponse
// ---------------------------------------------------------------------------

fn marking_row_to_response(row: &MarkingQueueRow) -> MarkingStatusResponse {
    MarkingStatusResponse {
        phase: row.phase as i32,
        progress: row.progress.clone(),
        error: row.error.clone(),
        estimated_completion: None,
    }
}

// ---------------------------------------------------------------------------
// Helper row types for SQL lookups in finalize_paper
// ---------------------------------------------------------------------------

#[derive(diesel::QueryableByName)]
struct SchoolInfoRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub motto: Option<String>,
}

#[derive(diesel::QueryableByName)]
struct ExamNameRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub name: String,
}

#[derive(diesel::QueryableByName)]
struct SubjectNameRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub name: String,
}

// ---------------------------------------------------------------------------
// Bulk import: helper structs for JSON parsing
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct BulkImportJson {
    subject: String,
    curriculum: String,
    grade: i32,
    topic: String,
    questions: Vec<BulkImportQuestion>,
}

#[derive(serde::Deserialize)]
struct BulkImportQuestion {
    text: String,
    marks: i32,
    #[serde(default)]
    example_answer: Option<String>,
    #[serde(default)]
    rubric: Vec<BulkImportRubric>,
}

#[derive(serde::Deserialize)]
struct BulkImportRubric {
    criterion: String,
    marks: i32,
}

// ---------------------------------------------------------------------------
// Helper row types for raw SQL lookups during bulk import
// ---------------------------------------------------------------------------

#[derive(diesel::QueryableByName)]
struct SubjectIdRow {
    #[diesel(sql_type = Integer)]
    pub id: i32,
}

#[derive(diesel::QueryableByName)]
struct TopicIdRow {
    #[diesel(sql_type = Integer)]
    pub id: i32,
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

        // Question catalog writes are global/system-wide and must not derive or require a school.
        // Validate inputs
        if req.text.trim().is_empty() {
            return Err(Error::InvalidQuestionText);
        }
        if req.marks <= 0 {
            return Err(Error::InvalidQuestionMarks);
        }

        // Validate rubric marks sum
        if !req.rubric.is_empty() {
            let rubric_sum: i32 = req.rubric.iter().map(|r| r.marks).sum();
            if rubric_sum != req.marks {
                return Err(Error::InvalidRubricMarks);
            }
        }

        let question = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();
            conn.transaction(|conn| {
                let qid = question_bank::insert_question(
                    conn,
                    req.topic_id,
                    &req.text,
                    req.marks as i16,
                    req.example_answer.as_deref(),
                    &user_id,
                )?;

                if !req.rubric.is_empty() {
                    let tuples = rubric_tuples(&req.rubric);
                    let refs: Vec<(i16, &str, i16)> = tuples
                        .iter()
                        .map(|(p, c, m)| (*p, c.as_str(), *m))
                        .collect();
                    question_bank::insert_rubric_criteria(conn, qid, &refs)?;
                }

                load_full_question(conn, qid, false)
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

        let text = req.text.as_deref();
        let marks = req.marks.map(|m| m as i16);
        let example_answer: Option<Option<&str>> = if req.example_answer.is_some() {
            Some(req.example_answer.as_deref())
        } else {
            None
        };

        let question = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();
            conn.transaction(|conn| {
                question_bank::update_question(conn, qid, text, marks, example_answer)?;

                if !req.rubric.is_empty() {
                    let tuples = rubric_tuples(&req.rubric);
                    let refs: Vec<(i16, &str, i16)> = tuples
                        .iter()
                        .map(|(p, c, m)| (*p, c.as_str(), *m))
                        .collect();
                    question_bank::replace_rubric_criteria(conn, qid, &refs)?;
                }

                load_full_question(conn, qid, false)
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
        CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();
            question_bank::delete_question(conn, req.question_id)
        })?;

        Ok(DeleteQuestionResponse {})
    }

    // ── bulk_import_questions ────────────────────────────────────────────

    async fn bulk_import_questions(
        &self,
        token: Token,
        req: BulkImportRequest,
    ) -> Result<BulkImportResponse> {
        let user_id = token.user.to_string();

        // Subject/topic/question catalog imports are global/system-wide and must not derive or require a school.
        let parsed: BulkImportJson = serde_json::from_str(&req.json_content).map_err(|e| {
            error!("bulk_import: JSON parse error: {e}");
            Error::InvalidBulkImportJson
        })?;

        // Map only explicit curriculum values; reject malformed imports instead of coercing.
        let curriculum_int: i16 = match parsed.curriculum.as_str() {
            "844" => 1,
            "cbc" => 0,
            _ => return Err(Error::InvalidCurriculum),
        };

        let result = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();
            conn.transaction(|conn| {
                // Look up subject by (name, curriculum)
                let subject_row: SubjectIdRow =
                    sql_query("SELECT id FROM subjects WHERE name = ? AND curriculum = ? LIMIT 1")
                        .bind::<Text, _>(&parsed.subject)
                        .bind::<SmallInt, _>(curriculum_int)
                        .get_result(conn)
                        .map_err(|_| {
                            error!(
                                "bulk_import: subject not found: {} / {}",
                                parsed.subject, parsed.curriculum
                            );
                            Error::SubjectNotFound
                        })?;

                // Look up topic by (name, subject, grade)
                let topic_row: TopicIdRow = sql_query(
                    "SELECT id FROM topics WHERE name = ? AND subject = ? AND grade = ? LIMIT 1",
                )
                .bind::<Text, _>(&parsed.topic)
                .bind::<Integer, _>(subject_row.id)
                .bind::<Integer, _>(parsed.grade)
                .get_result(conn)
                .map_err(|_| {
                    error!(
                        "bulk_import: topic not found: {} / subject={} / grade={}",
                        parsed.topic, subject_row.id, parsed.grade
                    );
                    Error::TopicNotFound
                })?;

                let mut created_count: i32 = 0;
                let mut errors: Vec<ImportError> = Vec::new();
                let mut question_ids: Vec<i32> = Vec::new();

                for (idx, q) in parsed.questions.iter().enumerate() {
                    // Validate marks
                    if q.marks <= 0 || q.text.trim().is_empty() {
                        errors.push(ImportError {
                            index: idx as i32,
                            message: "invalid text or marks".into(),
                        });
                        continue;
                    }

                    // Validate rubric sum
                    if !q.rubric.is_empty() {
                        let rubric_sum: i32 = q.rubric.iter().map(|r| r.marks).sum();
                        if rubric_sum != q.marks {
                            errors.push(ImportError {
                                index: idx as i32,
                                message: format!(
                                    "rubric marks sum ({}) != question marks ({})",
                                    rubric_sum, q.marks
                                ),
                            });
                            continue;
                        }
                    }

                    match question_bank::insert_question(
                        conn,
                        topic_row.id,
                        &q.text,
                        q.marks as i16,
                        q.example_answer.as_deref(),
                        &user_id,
                    ) {
                        Ok(qid) => {
                            if !q.rubric.is_empty() {
                                let tuples: Vec<(i16, &str, i16)> = q
                                    .rubric
                                    .iter()
                                    .enumerate()
                                    .map(|(i, r)| {
                                        ((i + 1) as i16, r.criterion.as_str(), r.marks as i16)
                                    })
                                    .collect();
                                if let Err(e) =
                                    question_bank::insert_rubric_criteria(conn, qid, &tuples)
                                {
                                    errors.push(ImportError {
                                        index: idx as i32,
                                        message: format!("rubric insert failed: {e}"),
                                    });
                                    continue;
                                }
                            }
                            question_ids.push(qid);
                            created_count += 1;
                        }
                        Err(e) => {
                            errors.push(ImportError {
                                index: idx as i32,
                                message: format!("insert failed: {e}"),
                            });
                        }
                    }
                }

                Ok::<_, Error>(BulkImportResponse {
                    questions_created: created_count,
                    errors,
                    question_ids,
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
        let urls = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();
            let mut result = Vec::with_capacity(req.images.len());

            for spec in &req.images {
                let ext = spec.filename.rsplit('.').next().unwrap_or("webp");
                let key = format!("questions/{}/{}.{}", spec.question_id, spec.position, ext);
                let put_url = sign::url(&key, sign::PUT_TTL, true);

                // Insert the image row in the DB
                question_bank::insert_question_image(
                    conn,
                    spec.question_id,
                    spec.position as i16,
                    spec.context as i16,
                    &key,
                    spec.caption.as_deref(),
                )?;

                result.push(ImageUploadUrl {
                    question_id: spec.question_id,
                    position: spec.position,
                    key,
                    put_url,
                });
            }

            Ok(result) as Result<Vec<ImageUploadUrl>>
        })?;

        Ok(ImageUploadUrlsResponse { urls })
    }

    // ── generate_paper ───────────────────────────────────────────────────

    async fn generate_paper(
        &self,
        _token: Token,
        req: GeneratePaperRequest,
    ) -> Result<GeneratePaperResponse> {
        let response = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();

            // Validate: sum of topic allocations == total_marks
            let alloc_sum: i32 = req.topic_allocations.iter().map(|ta| ta.marks).sum();
            if alloc_sum != req.total_marks {
                tracing::warn!(
                    "generate_paper: allocation marks ({}) != total marks ({})",
                    alloc_sum,
                    req.total_marks
                );
                return Err(Error::InvalidPermissions);
            }

            // For each topic allocation, select random questions
            let mut all_questions: Vec<(i32, i16)> = Vec::new();
            let mut position: i16 = 0;

            for alloc in &req.topic_allocations {
                let selected = question_bank::select_random_questions(
                    conn,
                    alloc.topic_id,
                    alloc.marks as i16,
                    &[],
                )?;

                let selected_marks: i32 = selected.iter().map(|q| q.marks as i32).sum();
                if selected_marks < alloc.marks {
                    tracing::warn!(
                        "generate_paper: not enough questions for topic {}: need {} marks, found {}",
                        alloc.topic_id,
                        alloc.marks,
                        selected_marks
                    );
                    return Err(Error::NothingToUpdate);
                }

                for q in &selected {
                    all_questions.push((q.id, position));
                    position += 1;
                }
            }

            // Delete any existing paper_questions for this paper
            question_bank::delete_paper_questions(
                conn,
                &req.school,
                &req.exam,
                req.subject,
                req.paper.map(|p| p as i16),
                req.grade as i16,
                req.stream.map(|s| s as i16),
            )?;

            // Insert new paper_questions
            question_bank::insert_paper_questions(
                conn,
                &req.school,
                &req.exam,
                req.subject,
                req.paper.map(|p| p as i16),
                req.grade as i16,
                req.stream.map(|s| s as i16),
                &all_questions,
            )?;

            // Build response with full question data
            let mut paper_questions = Vec::new();
            for (qid, pos) in &all_questions {
                let question = load_full_question(conn, *qid, true)?;
                paper_questions.push(PaperQuestion {
                    position: *pos as i32,
                    question: Some(question),
                });
            }

            Ok(GeneratePaperResponse {
                questions: paper_questions,
            })
        })?;

        Ok(response)
    }

    // ── regenerate_question ──────────────────────────────────────────────

    async fn regenerate_question(
        &self,
        _token: Token,
        req: RegenerateQuestionRequest,
    ) -> Result<RegenerateQuestionResponse> {
        let response = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();

            // Load current paper questions to build exclude list
            let current_pqs = question_bank::get_paper_questions(
                conn,
                &req.school,
                &req.exam,
                req.subject,
                req.paper.map(|p| p as i16),
                req.grade as i16,
                req.stream.map(|s| s as i16),
            )?;

            let mut exclude: Vec<i32> = current_pqs.iter().map(|pq| pq.question).collect();
            // Also exclude any explicitly provided IDs
            exclude.extend_from_slice(&req.exclude_ids);

            // Select a new random question for this topic + marks
            let candidates = question_bank::select_random_questions(
                conn,
                req.topic_id,
                req.marks as i16,
                &exclude,
            )?;

            let replacement = candidates.first().ok_or_else(|| {
                tracing::warn!("regenerate_question: no alternative questions available");
                Error::NothingToUpdate
            })?;

            // Replace at the given position
            question_bank::replace_paper_question_at_position(
                conn,
                &req.school,
                &req.exam,
                req.subject,
                req.paper.map(|p| p as i16),
                req.grade as i16,
                req.stream.map(|s| s as i16),
                req.position as i16,
                replacement.id,
            )?;

            let question = load_full_question(conn, replacement.id, true)?;

            Ok::<_, Error>(RegenerateQuestionResponse {
                replacement: Some(PaperQuestion {
                    position: req.position,
                    question: Some(question),
                }),
            })
        })?;

        Ok(response)
    }

    // ── edit_paper_question ──────────────────────────────────────────────

    async fn edit_paper_question(
        &self,
        _token: Token,
        req: EditPaperQuestionRequest,
    ) -> Result<EditPaperQuestionResponse> {
        let question = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();
            conn.transaction(|conn| {
                let text = req.text.as_deref();
                let marks = req.marks.map(|m| m as i16);
                let example_answer: Option<Option<&str>> = if req.example_answer.is_some() {
                    Some(req.example_answer.as_deref())
                } else {
                    None
                };

                // Update the question itself (persists to DB, improving the question bank)
                question_bank::update_question(conn, req.question_id, text, marks, example_answer)?;

                // Replace rubric if provided
                if !req.rubric.is_empty() {
                    let tuples = rubric_tuples(&req.rubric);
                    let refs: Vec<(i16, &str, i16)> = tuples
                        .iter()
                        .map(|(p, c, m)| (*p, c.as_str(), *m))
                        .collect();
                    question_bank::replace_rubric_criteria(conn, req.question_id, &refs)?;
                }

                // Load and return updated question
                load_full_question(conn, req.question_id, true)
            })
        })?;

        Ok(EditPaperQuestionResponse {
            question: Some(question),
        })
    }

    // ── finalize_paper ───────────────────────────────────────────────────

    async fn finalize_paper(
        &self,
        _token: Token,
        req: FinalizePaperRequest,
    ) -> Result<FinalizePaperResponse> {
        let (pdf_bytes, pdf_key) = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();

            // Load paper questions for this paper (ordered by position)
            let paper_qs = question_bank::get_paper_questions(
                conn,
                &req.school,
                &req.exam,
                req.subject,
                req.paper.map(|p| p as i16),
                req.grade as i16,
                req.stream.map(|s| s as i16),
            )?;

            if paper_qs.is_empty() {
                return Err(Error::NothingToUpdate);
            }

            // Load full question data for each paper question
            let mut questions_data: Vec<(String, i16, Vec<(String, i16)>)> = Vec::new();
            for pq in &paper_qs {
                let row = question_bank::get_question(conn, pq.question)?;
                let rubric = question_bank::get_rubric_criteria(conn, pq.question)?;
                questions_data.push((
                    row.text.clone(),
                    row.marks,
                    rubric
                        .iter()
                        .map(|r| (r.criterion.clone(), r.marks))
                        .collect(),
                ));
            }

            // Load school name + motto
            let school_info: SchoolInfoRow =
                sql_query("SELECT name, motto FROM schools WHERE id = ?")
                    .bind::<Text, _>(&req.school)
                    .get_result(conn)?;

            // Load exam name
            let exam_info: ExamNameRow = sql_query("SELECT name FROM exams WHERE id = ?")
                .bind::<Text, _>(&req.exam)
                .get_result(conn)?;

            // Load subject name
            let subject_info: SubjectNameRow = sql_query("SELECT name FROM subjects WHERE id = ?")
                .bind::<Integer, _>(req.subject)
                .get_result(conn)?;

            // Generate PDF
            let pdf_bytes = crate::pdf::generate_paper_pdf(
                &school_info.name,
                school_info.motto.as_deref(),
                &exam_info.name,
                &subject_info.name,
                req.paper.map(|p| p as i16),
                req.grade as i16,
                &questions_data,
            )
            .map_err(|e| {
                error!("PDF generation failed: {}", e);
                Error::Internal
            })?;

            // Build R2 key for the PDF
            let paper_suffix = req.paper.map(|p| format!("_{}", p)).unwrap_or_default();
            let stream_suffix = req.stream.map(|s| format!("_s{}", s)).unwrap_or_default();
            let pdf_key = format!(
                "schools/{}/exams/{}/papers/{}{}_{}{}/paper.pdf",
                req.school, req.exam, req.subject, paper_suffix, req.grade, stream_suffix
            );

            Ok((pdf_bytes, pdf_key))
        })?;

        // Upload PDF to R2 using presigned PUT URL
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

        let client = reqwest::Client::new();
        let resp = client
            .put(&put_url)
            .header("Content-Type", "application/pdf")
            .body(pdf_bytes)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to upload PDF to R2: {}", e);
                Error::Internal
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            error!(status = %status, body = %body, "R2 PDF upload failed");
            return Err(Error::Internal);
        }

        // Generate GET URL for download
        let get_url = sign::url(&pdf_key, sign::GET_TTL, false);
        let expiry = chrono::Utc::now().timestamp() + sign::GET_TTL as i64;

        Ok(FinalizePaperResponse {
            pdf_url: get_url,
            pdf_expiry: expiry,
        })
    }

    // ── get_paper_pdf ────────────────────────────────────────────────────

    async fn get_paper_pdf(
        &self,
        _token: Token,
        req: GetPaperPdfRequest,
    ) -> Result<GetPaperPdfResponse> {
        let paper_suffix = req.paper.map(|p| format!("_{}", p)).unwrap_or_default();
        let stream_suffix = req.stream.map(|s| format!("_s{}", s)).unwrap_or_default();
        let pdf_key = format!(
            "schools/{}/exams/{}/papers/{}{}_{}{}/paper.pdf",
            req.school, req.exam, req.subject, paper_suffix, req.grade, stream_suffix
        );

        let get_url = sign::url(&pdf_key, sign::GET_TTL, false);
        let expiry = chrono::Utc::now().timestamp() + sign::GET_TTL as i64;

        Ok(GetPaperPdfResponse {
            pdf_url: get_url,
            pdf_expiry: expiry,
        })
    }

    // ── list_questions ───────────────────────────────────────────────────

    async fn list_questions(
        &self,
        _token: Token,
        req: ListQuestionsRequest,
    ) -> Result<ListQuestionsResponse> {
        // Question catalog reads are global/system-wide and must not derive or require a school.
        let limit = if req.limit <= 0 { 50 } else { req.limit };
        let offset = if req.offset < 0 { 0 } else { req.offset };
        let min_marks = req.min_marks.map(|m| m as i16);
        let max_marks = req.max_marks.map(|m| m as i16);

        let (questions, total) = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();

            let (rows, total) = question_bank::list_questions(
                conn,
                req.topic_id,
                min_marks,
                max_marks,
                offset,
                limit,
            )?;

            let mut questions = Vec::with_capacity(rows.len());
            for row in &rows {
                let rubric = question_bank::get_rubric_criteria(conn, row.id)?;
                let images = question_bank::get_question_images(conn, row.id)?;
                questions.push(build_question_proto(row, &rubric, &images, true));
            }

            Ok((questions, total)) as Result<(Vec<Question>, i64)>
        })?;

        Ok(ListQuestionsResponse {
            questions,
            total: total as i32,
        })
    }

    // ── get_question ─────────────────────────────────────────────────────

    async fn get_question(
        &self,
        _token: Token,
        req: GetQuestionRequest,
    ) -> Result<GetQuestionResponse> {
        let question = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();
            load_full_question(conn, req.question_id, true)
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
        let grades = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();

            // Load the paper's questions to know which question IDs to look up
            let paper_qs = question_bank::get_paper_questions(
                conn,
                &req.school,
                &req.exam,
                req.subject,
                req.paper.map(|p| p as i16),
                req.grade as i16,
                req.stream.map(|s| s as i16),
            )?;

            if paper_qs.is_empty() {
                return Ok(Vec::new());
            }

            let question_ids: Vec<i32> = paper_qs.iter().map(|pq| pq.question).collect();

            let grade_rows = question_bank::get_question_grades_for_student(
                conn,
                &req.school,
                &req.exam,
                req.student,
                &question_ids,
            )?;

            let mut details = Vec::with_capacity(grade_rows.len());
            for gr in &grade_rows {
                // Load question text + rubric for each graded question
                let q_row = question_bank::get_question(conn, gr.question)?;
                let rubric = question_bank::get_rubric_criteria(conn, gr.question)?;

                details.push(QuestionGradeDetail {
                    question_id: gr.question,
                    question_text: q_row.text,
                    question_marks: q_row.marks as i32,
                    score: gr.score,
                    feedback: gr.feedback.clone(),
                    rubric: rubric
                        .iter()
                        .map(|r| RubricCriterion {
                            position: r.position as i32,
                            criterion: r.criterion.clone(),
                            marks: r.marks as i32,
                        })
                        .collect(),
                });
            }

            Ok(details) as Result<Vec<QuestionGradeDetail>>
        })?;

        Ok(GetQuestionGradesResponse { grades })
    }

    // ── get_marking_status ───────────────────────────────────────────────

    async fn get_marking_status(
        &self,
        _token: Token,
        req: MarkingStatusRequest,
    ) -> Result<MarkingStatusResponse> {
        let response = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();

            let row = question_bank::get_marking_status(
                conn,
                &req.school,
                &req.exam,
                req.subject,
                req.paper.map(|p| p as i16),
                req.grade as i16,
                req.stream.map(|s| s as i16),
            )?;

            match row {
                Some(r) => Ok::<_, Error>(marking_row_to_response(&r)),
                None => Ok::<_, Error>(MarkingStatusResponse {
                    phase: 0, // QUEUED default when no entry exists
                    progress: String::new(),
                    error: None,
                    estimated_completion: None,
                }),
            }
        })?;

        Ok(response)
    }
}
