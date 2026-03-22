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
use diesel::{Connection, RunQueryDsl};
use diesel::sql_query;
use diesel::sql_types::{BigInt, Integer, SmallInt, Text};
use std::sync::Arc;
use std::time::Instant;

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

        tracing::info!(
            school = %req.school,
            exam = %req.exam,
            subject = req.subject,
            paper = paper_num,
            scheme_count = req.scheme_count,
            student_count = req.students.len(),
            "request_upload_urls: generating presigned PUT URLs"
        );

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

        tracing::info!(
            school = %req.school,
            exam = %req.exam,
            scheme_url_count = scheme_urls.len(),
            student_url_count = student_urls.len(),
            "request_upload_urls: done"
        );

        Ok(UploadUrlsResponse { scheme_urls, student_urls })
    }

    async fn mark_paper(&self, _token: Token, req: MarkPaperRequest) -> Result<MarkPaperResponse> {
        let student_count = req.students.len();

        eprintln!(
            "[AI-SVC] mark_paper: entered (school={} exam={} subject={} students={} scheme_keys={})",
            req.school, req.exam, req.subject, student_count, req.scheme_keys.len()
        );
        tracing::info!(
            school = %req.school,
            exam = %req.exam,
            subject = req.subject,
            paper = ?req.paper,
            grade = req.grade,
            stream = ?req.stream,
            total_marks = req.total_marks,
            scheme_key_count = req.scheme_keys.len(),
            student_count = student_count,
            "mark_paper: RPC received"
        );

        // Generate GET URLs for all S3 keys (pure crypto, no network)
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

        eprintln!(
            "[AI-SVC] mark_paper: S3 GET URLs generated ({} scheme, {} students) — spawning background task",
            scheme_get_urls.len(), student_data.len()
        );
        tracing::debug!(
            school = %req.school,
            exam = %req.exam,
            scheme_get_url_count = scheme_get_urls.len(),
            student_data_count = student_data.len(),
            "mark_paper: S3 GET URLs generated — spawning background task"
        );

        let gemini = self.gemini.clone();
        let school = req.school.clone();
        let exam = req.exam.clone();
        let subject = req.subject;
        let paper = req.paper;

        tokio::spawn(async move {
            let task_start = Instant::now();
            eprintln!("[AI-TASK] started: school={} exam={} students={}", school, exam, student_count);
            tracing::info!(
                school = %school,
                exam = %exam,
                student_count = student_count,
                "ai_task: started — downloading images and calling Gemini"
            );

            match gemini.mark_paper(&scheme_get_urls, &student_data).await {
                Ok(scores) => {
                    eprintln!("[AI-TASK] Gemini returned {} scores in {}ms", scores.len(), task_start.elapsed().as_millis());
                    tracing::info!(
                        school = %school,
                        exam = %exam,
                        scored_count = scores.len(),
                        elapsed_ms = task_start.elapsed().as_millis(),
                        "ai_task: Gemini returned scores — writing grades to DB"
                    );

                    let school2 = school.clone();
                    let exam2 = exam.clone();
                    let result = tokio::task::spawn_blocking(move || {
                        let write_start = Instant::now();

                        // Fetch exam info for AI usage tracking (read-only, outside transaction)
                        let exam_info = fetch_exam_year_term(&school2, &exam2);
                        tracing::debug!(
                            school = %school2,
                            exam = %exam2,
                            exam_info_ok = exam_info.is_ok(),
                            "ai_task: exam year/term lookup for usage tracking"
                        );

                        // Write ALL grades + usage records in a SINGLE transaction.
                        // This acquires the write lock exactly once instead of N*3 times,
                        // eliminating the repeated lock contention with the sync service.
                        tracing::debug!(
                            school = %school2,
                            exam = %exam2,
                            student_count = scores.len(),
                            "ai_task: opening DB transaction"
                        );

                        let ops_result: std::result::Result<Vec<(i32, u8)>, Error> = CONN.with(|cell| {
                            let conn = &mut *cell.borrow_mut();
                            conn.transaction(|conn| {
                                let mut ops = Vec::with_capacity(scores.len());

                                for score in &scores {
                                    tracing::debug!(
                                        adm = score.adm,
                                        score = score.score,
                                        total = score.total,
                                        "ai_task: writing grade in transaction"
                                    );

                                    let grade = GradeInsert {
                                        school: school2.clone(),
                                        exam: exam2.clone(),
                                        student: score.adm,
                                        subject,
                                        paper,
                                        score: score.score as f32,
                                        total: score.total,
                                    };

                                    let row_key = format!(
                                        "{}|{}|{}|{}|{}",
                                        school2, exam2, score.adm, subject,
                                        paper.map(|v| v.to_string()).unwrap_or_default()
                                    );

                                    // Distinguish UNIQUE conflict from genuine errors
                                    let op = match insert::insert_grade(conn, &grade) {
                                        Ok(_) => OP_INSERT,
                                        Err(Error::Conflict) => {
                                            tracing::debug!(
                                                adm = score.adm,
                                                "ai_task: grade already exists, updating"
                                            );
                                            let update_payload = UpdateGradePayload {
                                                school: school2.clone(),
                                                exam: exam2.clone(),
                                                student: score.adm,
                                                subject,
                                                paper,
                                                score: Some(score.score as f32),
                                                total: Some(score.total),
                                            };
                                            update::update_grade(conn, &row_key, &update_payload)?;
                                            OP_UPDATE
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                adm = score.adm,
                                                error = %e,
                                                "ai_task: grade insert FAILED — rolling back all grades"
                                            );
                                            return Err(e);
                                        }
                                    };
                                    ops.push((score.adm, op));

                                    // Upsert AI usage inside the same transaction
                                    if let Ok((year, term)) = &exam_info {
                                        let now = chrono::Utc::now().timestamp();
                                        sql_query(
                                            "INSERT INTO aiusage (school, student, year, term, allocated, used, created, updated)                                              VALUES (?, ?, ?, ?, 0, 1, ?, ?)                                              ON CONFLICT (school, student, year, term)                                              DO UPDATE SET used = used + 1, updated = ?"
                                        )
                                        .bind::<Text, _>(&school2)
                                        .bind::<Integer, _>(score.adm)
                                        .bind::<Integer, _>(*year)
                                        .bind::<SmallInt, _>(*term)
                                        .bind::<BigInt, _>(now)
                                        .bind::<BigInt, _>(now)
                                        .bind::<BigInt, _>(now)
                                        .execute(conn)?;
                                    }
                                }

                                Ok(ops)
                            })
                        });

                        match ops_result {
                            Ok(ops) => {
                                eprintln!("[AI-TASK] DB transaction committed: {} grades written in {}ms", ops.len(), write_start.elapsed().as_millis());
                                tracing::info!(
                                    school = %school2,
                                    exam = %exam2,
                                    written_count = ops.len(),
                                    elapsed_ms = write_start.elapsed().as_millis(),
                                    "ai_task: DB transaction committed — appending changelog"
                                );

                                // Append changelog entries (file I/O, separate from DB transaction)
                                let log_user = Id::system();
                                LOG.with(|cell| {
                                    let mut log = cell.borrow_mut();
                                    for &(adm, op) in &ops {
                                        let record = Record::new(log_user, TBL_GRADES, op, 0);
                                        if let Err(e) = log.append(&record) {
                                            tracing::error!(
                                                adm = adm,
                                                error = %e,
                                                "ai_task: grades changelog append failed"
                                            );
                                        }
                                        let ai_record = Record::new(log_user, TBL_AIUSAGE, OP_UPDATE, 0);
                                        if let Err(e) = log.append(&ai_record) {
                                            tracing::error!(
                                                adm = adm,
                                                error = %e,
                                                "ai_task: aiusage changelog append failed"
                                            );
                                        }
                                    }
                                });

                                ops.len()
                            }
                            Err(e) => {
                                eprintln!("[AI-TASK] DB transaction FAILED after {}ms: {}", write_start.elapsed().as_millis(), e);
                                tracing::error!(
                                    school = %school2,
                                    exam = %exam2,
                                    error = %e,
                                    elapsed_ms = write_start.elapsed().as_millis(),
                                    "ai_task: DB transaction FAILED — no grades written"
                                );
                                0
                            }
                        }
                    })
                    .await;

                    match result {
                        Ok(count) => tracing::info!(
                            school = %school,
                            exam = %exam,
                            graded = count,
                            of = student_count,
                            total_elapsed_ms = task_start.elapsed().as_millis(),
                            "ai_task: COMPLETE"
                        ),
                        Err(e) => tracing::error!(
                            school = %school,
                            exam = %exam,
                            error = %e,
                            elapsed_ms = task_start.elapsed().as_millis(),
                            "ai_task: spawn_blocking PANICKED"
                        ),
                    }
                }
                Err(e) => {
                    eprintln!("[AI-TASK] Gemini FAILED after {}ms: {}", task_start.elapsed().as_millis(), e);
                    tracing::error!(
                        school = %school,
                        exam = %exam,
                        error = %e,
                        elapsed_ms = task_start.elapsed().as_millis(),
                        "ai_task: Gemini marking FAILED"
                    );
                }
            }
        });

        eprintln!("[AI-SVC] mark_paper: background task spawned — sending accepted=true to client");
        tracing::info!(
            school = %req.school,
            exam = %req.exam,
            student_count = student_count,
            "mark_paper: background task spawned — responding accepted=true"
        );

        Ok(MarkPaperResponse {
            accepted: true,
            message: format!("Marking {} students...", student_count),
        })
    }
}

/// Look up the year and term for an exam (read-only).
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
            tracing::error!(
                school = %school,
                exam = %exam,
                error = %e,
                "fetch_exam_year_term: DB query failed"
            );
            Error::Internal
        })?
        .into_iter()
        .next()
        .ok_or_else(|| {
            tracing::error!(
                school = %school,
                exam = %exam,
                "fetch_exam_year_term: exam not found"
            );
            Error::Internal
        })?;

    Ok((info.year, info.term))
}
