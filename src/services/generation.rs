#![allow(dead_code, unused_variables)]

use crate::config::storage::sign;
use crate::db::database::CONN;
use crate::db::database::tables::{
    paper_management as pm_db, papers as papers_db, question_bank as qb,
};
use crate::pdf::{PaperPart, PaperPdfInput, PaperQuestion as PdfQuestion};
use crate::types::error::Error;
use crate::types::id::Id;
use crate::types::paper::{
    GenerationMode, GenerationStatus, Paper, PaperSchedule, PaperStatus, PaperType, PaperUpdate,
};
use diesel::sql_query;
use diesel::sql_types::{Integer, Text};
use diesel::{OptionalExtension, RunQueryDsl};
use tracing::{error, info};

// ── SQL helper structs ────────────────────────────────────────────────────────

#[derive(diesel::QueryableByName)]
struct SchoolInfoRow {
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub motto: Option<String>,
}

#[derive(diesel::QueryableByName)]
struct SubjectNameRow {
    #[diesel(sql_type = Text)]
    pub name: String,
}

#[derive(diesel::QueryableByName)]
struct StudentNameRow {
    #[diesel(sql_type = Text)]
    pub name: String,
}

#[derive(diesel::QueryableByName)]
struct EventSchoolRow {
    #[diesel(sql_type = Text)]
    pub school: String,
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Load questions for a paper (and optional student) from the DB and build the
/// `Vec<PdfQuestion>` structure required by the PDF engine.
fn load_pdf_questions(
    paper_id: &str,
    student: Option<i32>,
) -> crate::types::error::Result<Vec<PdfQuestion>> {
    CONN.with(|conn| {
        let pqs = qb::get_paper_questions(conn, paper_id, student)?;
        let mut result: Vec<PdfQuestion> = Vec::with_capacity(pqs.len());

        for pq in &pqs {
            let q = qb::get_question(conn, pq.question)?.ok_or(Error::NotFound)?;
            let rubric = qb::get_rubric_criteria(conn, pq.question)?;
            let parts = qb::get_question_parts(conn, pq.question)?;

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

            result.push(PdfQuestion {
                body: q.body.clone(),
                body_format: q.body_format as u8,
                marks: q.marks,
                max_marks: q.max_marks,
                answer_space_type: q.answer_space_type as u8,
                answer_lines: q.answer_lines,
                answer_box_height_mm: q.answer_box_height_mm,
                stimulus: q.stimulus.clone(),
                example_answer: q.example_answer.clone(),
                rubric: rubric_data,
                parts: pdf_parts,
                section: pq.section.clone(),
            });
        }

        Ok(result)
    })
}

/// Upload a PDF byte buffer to R2 at the given object key.
async fn upload_to_r2(key: &str, bytes: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let put_url = sign::presign(
        env!("R2_ACCOUNT_ID"),
        env!("R2_BUCKET"),
        env!("R2_ACCESS_KEY_ID"),
        env!("R2_SECRET_ACCESS_KEY"),
        "PUT",
        key,
        sign::PUT_TTL,
        Some("application/pdf"),
    );

    let client = reqwest::Client::new();
    let resp = client
        .put(&put_url)
        .header("Content-Type", "application/pdf")
        .body(bytes)
        .send()
        .await
        .map_err(|e| format!("R2 PUT request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("R2 PUT returned non-success status: {}", resp.status()).into());
    }

    Ok(())
}

// ── Public scheduler ──────────────────────────────────────────────────────────

/// Background task — spawned at server startup. Polls every 30 seconds for:
///   1. Exam paper schedules that are due for generation.
///   2. Finalized papers whose `reveal_at` timestamp has passed (auto-reveal).
pub async fn run_generation_scheduler() {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

        // ── Poll 1: pending exam paper generation ────────────────────────────
        let due_schedules =
            CONN.with(|conn| pm_db::get_pending_generation(conn));

        match due_schedules {
            Ok(schedules) => {
                for schedule in schedules {
                    tokio::spawn(generate_exam_paper(schedule));
                }
            }
            Err(e) => error!("generation scheduler: poll error: {e}"),
        }

        // ── Poll 2: auto-reveal papers past their reveal_at ──────────────────
        let reveal_due =
            CONN.with(|conn| papers_db::get_papers_due_for_reveal(conn));

        match reveal_due {
            Ok(paper_ids) => {
                for paper_id in paper_ids {
                    if let Err(e) = CONN.with(|conn| {
                        papers_db::transition_paper_status(
                            conn,
                            &paper_id,
                            PaperStatus::Revealed,
                        )
                    }) {
                        error!("auto-reveal failed for paper {paper_id}: {e}");
                    }
                }
            }
            Err(e) => error!("generation scheduler: reveal poll error: {e}"),
        }
    }
}

// ── Exam paper generation ─────────────────────────────────────────────────────

/// Entry point for exam paper generation spawned by the scheduler.
/// Marks the schedule as Generating, delegates to the inner function, and
/// marks it as Generated or Failed depending on the outcome.
async fn generate_exam_paper(schedule: PaperSchedule) {
    let schedule_id = schedule.id.to_string();

    // Mark as Generating
    if let Err(e) = CONN.with(|conn| {
        pm_db::set_generation_status(
            conn,
            &schedule_id,
            GenerationStatus::Generating,
            None,
        )
    }) {
        error!("generate_exam_paper: failed to set Generating for {schedule_id}: {e}");
        return;
    }

    match do_generate_exam_paper(&schedule).await {
        Ok(()) => {
            info!("generate_exam_paper: succeeded for schedule {schedule_id}");
        }
        Err(e) => {
            error!("generate_exam_paper: failed for schedule {schedule_id}: {e}");
            let _ = CONN.with(|conn| {
                pm_db::set_generation_status(
                    conn,
                    &schedule_id,
                    GenerationStatus::Failed,
                    Some(&e.to_string()),
                )
            });
        }
    }
}

/// Core exam paper generation logic:
///   1. Verify exam coverage topics exist.
///   2. Resolve school from the event.
///   3. Load school / subject names for the PDF header.
///   4. Create a Paper record (Draft).
///   5. Select questions for each topic (equal mark split).
///   6. Insert paper_questions.
///   7. Transition paper → QuestionsSet.
///   8. Build PDF + marking scheme.
///   9. Upload both to R2.
///  10. Store keys on the paper record and transition → Finalized.
///  11. Link the paper to the schedule.
///  12. Mark the schedule as Generated.
async fn do_generate_exam_paper(
    schedule: &PaperSchedule,
) -> Result<(), Box<dyn std::error::Error>> {
    let schedule_id = schedule.id.to_string();

    // 1. Confirmed exam coverage topics
    let topic_ids =
        CONN.with(|conn| pm_db::get_exam_coverage(conn, &schedule_id))?;

    if topic_ids.is_empty() {
        return Err("no exam coverage confirmed for this schedule".into());
    }

    // 2. Resolve school from the event
    let school = CONN.with(|conn| {
        let row: Option<EventSchoolRow> = sql_query("SELECT school FROM events WHERE id = ?")
            .bind::<Text, _>(&schedule.event)
            .get_result(conn)
            .optional()
            .map_err(Error::internal)?;
        row.ok_or(Error::EventNotFound).map(|r| r.school)
    })?;

    // 3. Load school name / motto and subject name for the PDF header
    let (school_name, school_motto, subject_name) = CONN.with(|conn| {

        let school_info: Option<SchoolInfoRow> =
            sql_query("SELECT name, motto FROM schools WHERE id = ?")
                .bind::<Text, _>(&school)
                .get_result(conn)
                .optional()
                .map_err(Error::internal)?;
        let school_name = school_info
            .as_ref()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "School".into());
        let school_motto = school_info.as_ref().and_then(|s| s.motto.clone());

        let subject_info: Option<SubjectNameRow> =
            sql_query("SELECT name FROM subjects WHERE id = ?")
                .bind::<Integer, _>(schedule.subject)
                .get_result(conn)
                .optional()
                .map_err(Error::internal)?;
        let subject_name = subject_info
            .map(|s| s.name)
            .unwrap_or_else(|| "Subject".into());

        Ok::<_, Error>((school_name, school_motto, subject_name))
    })?;

    // 4. Create Paper record (status = Draft)
    let now = chrono::Utc::now().timestamp();
    let paper_id_val = Id::default();
    let paper_id_str = paper_id_val.to_string();
    let paper_name = format!("{subject_name} Paper");

    let paper = CONN.with(|conn| {
        let new_paper = Paper {
            id: paper_id_val,
            school: school.clone(),
            event: Some(schedule.event.clone()),
            subject: schedule.subject,
            grade: schedule.grade,
            stream: schedule.stream,
            type_: PaperType::Exam,
            teacher: Id::system().to_string(),
            name: paper_name.clone(),
            total_marks: 80,
            duration_minutes: schedule.duration_minutes,
            date: schedule.date,
            status: PaperStatus::Draft,
            pdf_key: None,
            ms_key: None,
            generation_mode: GenerationMode::ClassUniform,
            instructions: None,
            created: now,
            updated: now,
        };
        papers_db::insert_paper(conn, &new_paper)
    })?;

    // 5. Select questions for each topic.
    let all_questions: Vec<(i32, i16)> = CONN.with(|conn| {
        let mut questions: Vec<(i32, i16)> = Vec::new();
        let mut position: i16 = 0;

        for topic_id in &topic_ids {
            let selected =
                qb::select_questions_for_paper(conn, *topic_id, &[], None)?;
            for q in &selected {
                if let Some(id) = q.id {
                    questions.push((id, position));
                    position += 1;
                }
            }
        }
        Ok::<_, Error>(questions)
    })?;

    if all_questions.is_empty() {
        return Err("no questions found for the confirmed exam coverage topics".into());
    }

    // 6. Insert paper questions (class-wide, student = None)
    CONN.with(|conn| {
        qb::insert_paper_questions(conn, &paper_id_str, None, &all_questions)
    })?;

    // 7. Transition → QuestionsSet
    CONN.with(|conn| {
        papers_db::transition_paper_status(
            conn,
            &paper_id_str,
            PaperStatus::QuestionsSet,
        )
    })?;

    // 8. Load questions and build PDF structures
    let pdf_questions = load_pdf_questions(&paper_id_str, None)?;

    let pdf_input = PaperPdfInput {
        school_name: &school_name,
        school_motto: school_motto.as_deref(),
        paper_name: &paper_name,
        subject_name: &subject_name,
        paper_number: None,
        grade: paper.grade,
        duration_minutes: Some(paper.duration_minutes),
        instructions: paper.instructions.as_deref(),
        questions: &pdf_questions,
    };

    let pdf_bytes = crate::pdf::generate_paper_pdf_typst(&pdf_input)
        .map_err(|e| format!("exam paper PDF generation failed: {e}"))?;
    let ms_bytes = crate::pdf::generate_marking_scheme_pdf_typst(&pdf_input)
        .map_err(|e| format!("marking scheme PDF generation failed: {e}"))?;

    // 9. Upload PDFs to R2
    let pdf_key = format!("papers/{paper_id_str}/paper.pdf");
    let ms_key = format!("papers/{paper_id_str}/marking_scheme.pdf");
    upload_to_r2(&pdf_key, pdf_bytes).await?;
    upload_to_r2(&ms_key, ms_bytes).await?;

    // 10. Store keys on paper and transition → Finalized
    CONN.with(|conn| {
        let now2 = chrono::Utc::now().timestamp();
        papers_db::update_paper(
            conn,
            &paper_id_str,
            PaperUpdate {
                pdf_key: Some(Some(pdf_key.clone())),
                ms_key: Some(Some(ms_key.clone())),
                updated: Some(now2),
                ..PaperUpdate::default()
            },
        )?;
        papers_db::transition_paper_status(conn, &paper_id_str, PaperStatus::Finalized)
    })?;

    // 11. Link paper to schedule
    CONN.with(|conn| {
        pm_db::link_paper_to_schedule(conn, &schedule_id, &paper_id_str)
    })?;

    // 12. Mark schedule as Generated
    CONN.with(|conn| {
        pm_db::set_generation_status(
            conn,
            &schedule_id,
            GenerationStatus::Generated,
            None,
        )
    })?;

    info!(
        "exam paper {paper_id_str} generated for schedule {schedule_id} \
         (pdf={pdf_key}, ms={ms_key})"
    );
    Ok(())
}

// ── Per-student paper generation ──────────────────────────────────────────────

/// Public entry point — generates a personalised PDF for one student.
/// Errors are logged; callers do not need to handle them.
pub async fn generate_per_student_paper(paper_id: &str, student_adm: i32) {
    if let Err(e) = do_generate_per_student_paper(paper_id, student_adm).await {
        error!("per-student generation failed: paper={paper_id} student={student_adm}: {e}");
    }
}

async fn do_generate_per_student_paper(
    paper_id: &str,
    student_adm: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load paper metadata
    let paper = CONN
        .with(|conn| papers_db::get_paper(conn, paper_id))?
        .ok_or(Error::PaperNotFound)?;

    // Determine whether per-student question overrides exist; fall back to
    // class-wide questions if none have been set for this student.
    let per_student_qs = CONN.with(|conn| {
        qb::get_paper_questions(conn, paper_id, Some(student_adm))
    })?;
    let effective_student = if per_student_qs.is_empty() {
        None
    } else {
        Some(student_adm)
    };

    // Load school name / motto, subject name, and student name
    let (school_name, school_motto, subject_name, student_name) = CONN.with(|conn| {

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

        let subject_info: Option<SubjectNameRow> =
            sql_query("SELECT name FROM subjects WHERE id = ?")
                .bind::<Integer, _>(paper.subject)
                .get_result(conn)
                .optional()
                .map_err(Error::internal)?;
        let subject_name = subject_info
            .map(|s| s.name)
            .unwrap_or_else(|| "Subject".into());

        let student_info: Option<StudentNameRow> =
            sql_query("SELECT name FROM students WHERE school = ? AND adm = ?")
                .bind::<Text, _>(&paper.school)
                .bind::<Integer, _>(student_adm)
                .get_result(conn)
                .optional()
                .map_err(Error::internal)?;
        let student_name = student_info
            .map(|s| s.name)
            .unwrap_or_else(|| "Student".to_string());

        Ok::<_, Error>((school_name, school_motto, subject_name, student_name))
    })?;

    // Build the PDF question list
    let pdf_questions = load_pdf_questions(paper_id, effective_student)?;

    let pdf_input = PaperPdfInput {
        school_name: &school_name,
        school_motto: school_motto.as_deref(),
        paper_name: &paper.name,
        subject_name: &subject_name,
        paper_number: None,
        grade: paper.grade,
        duration_minutes: Some(paper.duration_minutes),
        instructions: paper.instructions.as_deref(),
        questions: &pdf_questions,
    };

    // Generate personalised PDF with the actual student name
    let pdf_bytes = crate::pdf::generate_student_paper_pdf(&pdf_input, &student_name, student_adm)
        .map_err(|e| format!("per-student PDF generation failed: {e}"))?;

    // Upload to R2
    let key = format!("papers/{paper_id}/students/{student_adm}.pdf");
    upload_to_r2(&key, pdf_bytes).await?;

    // Record the key in the DB
    CONN.with(|conn| {
        papers_db::upsert_student_pdf_key(conn, paper_id, student_adm, &key)
    })?;

    info!("per-student PDF stored: paper={paper_id} student={student_adm} key={key}");
    Ok(())
}

// ── Enqueue helpers ───────────────────────────────────────────────────────────

/// Generate per-student PDFs for an Assessment paper.
/// Waits for all student PDFs to complete before returning so the caller
/// (e.g. the download flow) can rely on them being ready.
pub async fn enqueue_assessment(paper_id: &str) {
    let paper_id = paper_id.to_string();
    let students = match CONN.with(|conn| {
        let paper = papers_db::get_paper(conn, &paper_id)?.ok_or(Error::PaperNotFound)?;
        papers_db::get_enrolled_students(conn, &paper.school, paper.grade, paper.stream)
    }) {
        Ok(list) => list,
        Err(e) => {
            error!("enqueue_assessment: failed to load enrolled students: {e}");
            return;
        }
    };

    let mut handles = Vec::with_capacity(students.len());
    for student in students {
        let pid = paper_id.clone();
        handles.push(tokio::spawn(async move {
            generate_per_student_paper(&pid, student).await;
        }));
    }
    for handle in handles {
        let _ = handle.await;
    }
}

/// Enqueue per-student generation for an Assignment paper.
/// Delegates to the same logic as `enqueue_assessment`.
pub async fn enqueue_assignment(paper_id: &str) {
    enqueue_assessment(paper_id).await;
}

// ── ClassUniform finalisation ─────────────────────────────────────────────────

/// For ClassUniform mode: generate a named PDF for every enrolled student using
/// the class-wide question set.  Called by PaperManagementService.
pub async fn finalize_student_papers_job(paper_id: &str) {
    let paper_id = paper_id.to_string();
    tokio::spawn(async move {
        let students = CONN.with(|conn| {
            let paper = papers_db::get_paper(conn, &paper_id)?.ok_or(Error::PaperNotFound)?;
            papers_db::get_enrolled_students(conn, &paper.school, paper.grade, paper.stream)
        });
        match students {
            Ok(list) => {
                for student in list {
                    let pid = paper_id.clone();
                    tokio::spawn(async move {
                        generate_per_student_paper(&pid, student).await;
                    });
                }
            }
            Err(e) => error!("finalize_student_papers_job: failed to load enrolled students: {e}"),
        }
    });
}
