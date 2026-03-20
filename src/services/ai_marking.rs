use crate::ai::gemini::GeminiClient;
use crate::config::storage::sign;
use crate::db::changelog::{LOG, Record};
use crate::db::database::CONN;
use crate::db::database::tables::{insert, update};
use crate::proto::services::ai_marking::*;
use crate::proto::services::sync::{GradeInsert, UpdateGradePayload};
use crate::types::error::{Error, Result};
use crate::types::id::Id;
use crate::types::token::Token;
use diesel::RunQueryDsl;
use diesel::sql_query;
use diesel::sql_types::{BigInt, Integer, SmallInt, Text};
use std::sync::Arc;

const TBL_GRADES: u8 = 18;
const TBL_AIUSAGE: u8 = 24;
const OP_INSERT: u8 = 0;
const OP_UPDATE: u8 = 1;

pub struct AiMarkingService<C> {
    #[allow(dead_code)]
    config: Arc<C>,
    gemini: GeminiClient,
}

impl<C: Send + Sync + 'static> AiMarking for AiMarkingService<C> {
    type Config = Arc<C>;

    fn new(config: Self::Config) -> AiMarkingServer<Self> {
        AiMarkingServer::new(Self {
            config,
            gemini: GeminiClient::new(),
        })
    }

    async fn request_upload_urls(
        &self,
        _token: Token,
        req: UploadUrlsRequest,
    ) -> Result<UploadUrlsResponse> {
        let paper_num = req.paper.unwrap_or(0);

        // Generate PUT URLs for scheme images
        let scheme_urls: Vec<SignedUrl> = (0..req.scheme_count)
            .map(|i| {
                let key = format!(
                    "schools/{}/exams/{}/papers/{}_{}/scheme/{}",
                    req.school, req.exam, req.subject, paper_num, i
                );
                let url = sign::url(&key, sign::PUT_TTL, true);
                SignedUrl { key, url }
            })
            .collect();

        // Generate PUT URLs for student answer sheets
        let student_urls: Vec<StudentSignedUrls> = req
            .students
            .iter()
            .map(|s| {
                let urls: Vec<SignedUrl> = (0..s.count)
                    .map(|i| {
                        let key = format!(
                            "schools/{}/exams/{}/papers/{}_{}/students/{}/{}",
                            req.school, req.exam, req.subject, paper_num, s.adm, i
                        );
                        let url = sign::url(&key, sign::PUT_TTL, true);
                        SignedUrl { key, url }
                    })
                    .collect();
                StudentSignedUrls { adm: s.adm, urls }
            })
            .collect();

        Ok(UploadUrlsResponse {
            scheme_urls,
            student_urls,
        })
    }

    async fn mark_paper(&self, _token: Token, req: MarkPaperRequest) -> Result<MarkPaperResponse> {
        // Generate GET URLs for all S3 keys
        let scheme_get_urls: Vec<String> = req
            .scheme_keys
            .iter()
            .map(|key| sign::url(key, sign::GET_TTL, false))
            .collect();

        let student_data: Vec<(i32, Vec<String>)> = req
            .students
            .iter()
            .map(|s| {
                let urls: Vec<String> = s
                    .keys
                    .iter()
                    .map(|key| sign::url(key, sign::GET_TTL, false))
                    .collect();
                (s.adm, urls)
            })
            .collect();

        let student_count = student_data.len();

        // Clone values for the spawned task
        let gemini = self.gemini.clone();
        let total_marks = req.total_marks;
        let school = req.school.clone();
        let exam = req.exam.clone();
        let subject = req.subject;
        let paper = req.paper;

        // Spawn async marking task (return immediately to client)
        tokio::spawn(async move {
            match gemini
                .mark_paper(&scheme_get_urls, &student_data, total_marks)
                .await
            {
                Ok(scores) => {
                    // Write grades on a blocking thread (DB access is thread-local)
                    let school2 = school.clone();
                    let exam2 = exam.clone();
                    let result = tokio::task::spawn_blocking(move || {
                        // Look up exam year/term for AI usage tracking
                        let exam_info = fetch_exam_year_term(&school2, &exam2);

                        for score in &scores {
                            if let Err(e) = write_ai_grade(
                                &school2,
                                &exam2,
                                score.adm,
                                subject,
                                paper,
                                score.score,
                                total_marks,
                            ) {
                                tracing::error!(
                                    "Failed to write AI grade for student {}: {}",
                                    score.adm,
                                    e
                                );
                            }

                            // Track AI usage per student
                            if let Ok((year, term)) = &exam_info {
                                if let Err(e) = write_ai_usage(&school2, score.adm, *year, *term) {
                                    tracing::error!(
                                        "Failed to write AI usage for student {}: {}",
                                        score.adm,
                                        e
                                    );
                                }
                            }
                        }
                        scores.len()
                    })
                    .await;

                    match result {
                        Ok(count) => {
                            tracing::info!(
                                "AI marking complete: {}/{} students scored for school={} exam={}",
                                count,
                                student_count,
                                school,
                                exam
                            );
                        }
                        Err(e) => {
                            tracing::error!("Grade write task panicked: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Gemini marking failed for school={} exam={}: {}",
                        school,
                        exam,
                        e
                    );
                }
            }
        });

        Ok(MarkPaperResponse {
            accepted: true,
            message: format!("Marking {} students...", student_count),
        })
    }
}

/// Write a single AI-generated grade to the database and append a changelog entry.
fn write_ai_grade(
    school: &str,
    exam: &str,
    student: i32,
    subject: i32,
    paper: Option<i32>,
    score: f64,
    total: i32,
) -> Result<()> {
    let log_user = Id::system();

    let grade = GradeInsert {
        school: school.to_string(),
        exam: exam.to_string(),
        student,
        subject,
        paper,
        score: score as f32,
        total,
    };

    let row_key = format!(
        "{}|{}|{}|{}|{}",
        school,
        exam,
        student,
        subject,
        paper.map(|v| v.to_string()).unwrap_or_default()
    );

    // Upsert: try insert, on conflict update
    let inserted = CONN.with(|cell| insert::insert_grade(&mut *cell.borrow_mut(), &grade));
    if inserted.is_err() {
        // Row exists — update score/total
        let update_payload = UpdateGradePayload {
            school: school.to_string(),
            exam: exam.to_string(),
            student,
            subject,
            paper,
            score: Some(score as f32),
            total: Some(total),
        };
        CONN.with(|cell| update::update_grade(&mut *cell.borrow_mut(), &row_key, &update_payload))?;
        let record = Record::new(log_user, TBL_GRADES, OP_UPDATE, 0);
        LOG.with(|cell| cell.borrow_mut().append(&record))
            .map_err(|e| {
                tracing::error!("changelog append failed: {e}");
                Error::Internal
            })?;
    } else {
        let record = Record::new(log_user, TBL_GRADES, OP_INSERT, 0);
        LOG.with(|cell| cell.borrow_mut().append(&record))
            .map_err(|e| {
                tracing::error!("changelog append failed: {e}");
                Error::Internal
            })?;
    }

    Ok(())
}

/// Look up the year and term for an exam.
fn fetch_exam_year_term(school: &str, exam: &str) -> Result<(i32, i16)> {
    #[derive(diesel::QueryableByName)]
    struct ExamInfo {
        #[diesel(sql_type = Integer)]
        year: i32,
        #[diesel(sql_type = SmallInt)]
        term: i16,
    }

    let info = CONN
        .with(|cell| {
            sql_query("SELECT year, term FROM exams WHERE id = ? AND school = ?")
                .bind::<Text, _>(exam)
                .bind::<Text, _>(school)
                .load::<ExamInfo>(&mut *cell.borrow_mut())
        })
        .map_err(|e| {
            tracing::error!("fetch_exam_year_term failed: {e}");
            Error::Internal
        })?
        .into_iter()
        .next()
        .ok_or_else(|| {
            tracing::error!("exam not found: school={school} exam={exam}");
            Error::Internal
        })?;

    Ok((info.year, info.term))
}

/// Upsert AI usage: increment `used` count for a student in a given term.
/// If the row doesn't exist, create it with used=1, allocated=0.
/// If it exists, increment used by 1.
fn write_ai_usage(school: &str, student: i32, year: i32, term: i16) -> Result<()> {
    let log_user = Id::system();
    let now = chrono::Utc::now().timestamp();

    // Try INSERT ON CONFLICT to atomically upsert
    CONN.with(|cell| {
        sql_query(
            "INSERT INTO aiusage (school, student, year, term, allocated, used, created, updated) \
             VALUES (?, ?, ?, ?, 0, 1, ?, ?) \
             ON CONFLICT (school, student, year, term) \
             DO UPDATE SET used = used + 1, updated = ?",
        )
        .bind::<Text, _>(school)
        .bind::<Integer, _>(student)
        .bind::<Integer, _>(year)
        .bind::<SmallInt, _>(term)
        .bind::<BigInt, _>(now)
        .bind::<BigInt, _>(now)
        .bind::<BigInt, _>(now)
        .execute(&mut *cell.borrow_mut())
    })
    .map_err(|e| {
        tracing::error!("write_ai_usage upsert failed: {e}");
        Error::Internal
    })?;

    // Append changelog entry
    let record = Record::new(log_user, TBL_AIUSAGE, OP_UPDATE, 0);
    LOG.with(|cell| cell.borrow_mut().append(&record))
        .map_err(|e| {
            tracing::error!("changelog append for aiusage failed: {e}");
            Error::Internal
        })?;

    Ok(())
}
