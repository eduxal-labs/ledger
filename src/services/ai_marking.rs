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
use diesel::sql_query;
use diesel::sql_types::{BigInt, Integer, SmallInt, Text};
use diesel::{Connection, RunQueryDsl};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

const TBL_GRADES: u8 = 18;
const TBL_AIUSAGE: u8 = 24;
const OP_INSERT: u8 = 0;
const OP_UPDATE: u8 = 1;

/// Max concurrent AI API requests when marking students within one paper.
const MAX_CONCURRENT: usize = 4;

/// Queue capacity for pending mark requests.
const QUEUE_CAPACITY: usize = 32;

// ---------------------------------------------------------------------------
// Request types for the channel queue
// ---------------------------------------------------------------------------

struct MarkRequest {
    school: String,
    exam: String,
    subject: i32,
    paper: Option<i32>,
    scheme_get_urls: Vec<String>,
    students: Vec<(i32, Vec<String>)>, // (adm, [S3 GET URLs])
}

struct PreparedRequest {
    mark_req: MarkRequest,
    cache_name: Option<String>, // None = fallback to non-cached mode
    student_images: Vec<(i32, Vec<String>)>, // (adm, [base64 images])
}

// ---------------------------------------------------------------------------
// Service struct
// ---------------------------------------------------------------------------

pub struct AiMarkingService<C> {
    #[allow(dead_code)]
    config: Arc<C>,
    tx: mpsc::Sender<MarkRequest>,
}

impl<C: Send + Sync + 'static> AiMarking for AiMarkingService<C> {
    type Config = Arc<C>;

    fn new(config: Self::Config) -> AiMarkingServer<Self> {
        let (tx, rx) = mpsc::channel::<MarkRequest>(QUEUE_CAPACITY);
        let client = GeminiClient::new();

        spawn_marking_worker(rx, client);

        AiMarkingServer::new(Self { config, tx })
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

        Ok(UploadUrlsResponse {
            scheme_urls,
            student_urls,
        })
    }

    async fn mark_paper(&self, _token: Token, req: MarkPaperRequest) -> Result<MarkPaperResponse> {
        let student_count = req.students.len();

        eprintln!(
            "[AI-SVC] mark_paper: entered (school={} exam={} subject={} students={} scheme_keys={})",
            req.school,
            req.exam,
            req.subject,
            student_count,
            req.scheme_keys.len()
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

        let students: Vec<(i32, Vec<String>)> = req
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

        let mark_req = MarkRequest {
            school: req.school.clone(),
            exam: req.exam.clone(),
            subject: req.subject,
            paper: req.paper,
            scheme_get_urls,
            students,
        };

        // Push to queue — if full, return "server busy"
        self.tx.try_send(mark_req).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => {
                eprintln!("[AI-SVC] mark_paper: queue full — rejecting request");
                tracing::warn!(
                    school = %req.school,
                    exam = %req.exam,
                    "mark_paper: queue full — rejecting"
                );
                Error::SlowDown
            }
            mpsc::error::TrySendError::Closed(_) => {
                eprintln!("[AI-SVC] mark_paper: worker channel closed!");
                tracing::error!("mark_paper: worker channel closed");
                Error::Internal
            }
        })?;

        eprintln!("[AI-SVC] mark_paper: queued — sending accepted=true to client");
        tracing::info!(
            school = %req.school,
            exam = %req.exam,
            student_count = student_count,
            "mark_paper: queued — responding accepted=true"
        );

        Ok(MarkPaperResponse {
            accepted: true,
            message: format!("Marking {} students...", student_count),
        })
    }
}

// ---------------------------------------------------------------------------
// Background marking worker with pipeline prefetching
// ---------------------------------------------------------------------------

fn spawn_marking_worker(rx: mpsc::Receiver<MarkRequest>, gemini: GeminiClient) {
    tokio::spawn(async move {
        let mut rx = rx;
        let mut prefetched: Option<PreparedRequest> = None;

        eprintln!("[AI-WORKER] marking worker started");
        tracing::info!("ai_worker: marking worker started");

        loop {
            // Get next prepared request: from prefetch buffer or by waiting for a new one
            let current = if let Some(prepared) = prefetched.take() {
                eprintln!(
                    "[AI-WORKER] using prefetched request (school={})",
                    prepared.mark_req.school
                );
                tracing::info!(
                    school = %prepared.mark_req.school,
                    "ai_worker: using prefetched request"
                );
                prepared
            } else {
                // Block until a request arrives
                let req = match rx.recv().await {
                    Some(r) => r,
                    None => {
                        eprintln!("[AI-WORKER] channel closed — shutting down");
                        tracing::info!("ai_worker: channel closed — shutting down");
                        break;
                    }
                };
                eprintln!(
                    "[AI-WORKER] received request (school={} exam={} students={})",
                    req.school,
                    req.exam,
                    req.students.len()
                );
                tracing::info!(
                    school = %req.school,
                    exam = %req.exam,
                    student_count = req.students.len(),
                    "ai_worker: received request — preparing"
                );
                prepare(&gemini, req).await
            };

            // Try to grab next request non-blocking for prefetching
            let maybe_next = rx.try_recv().ok();
            let has_next = maybe_next.is_some();

            if has_next {
                eprintln!("[AI-WORKER] prefetching next request in parallel with marking");
                tracing::info!("ai_worker: prefetching next request in parallel");
            }

            // Run in parallel: mark current + prepare next (if any)
            let gemini_for_prefetch = gemini.clone();
            let (mark_result, prepared_next) =
                tokio::join!(mark_and_write(&gemini, current), async {
                    match maybe_next {
                        Some(next_req) => Some(prepare(&gemini_for_prefetch, next_req).await),
                        None => None,
                    }
                });

            // Handle mark result
            match mark_result {
                Ok(count) => {
                    eprintln!("[AI-WORKER] marking complete: {} grades written", count);
                    tracing::info!(grades_written = count, "ai_worker: marking complete");
                }
                Err(e) => {
                    eprintln!("[AI-WORKER] marking failed: {}", e);
                    tracing::error!(error = %e, "ai_worker: marking failed");
                }
            }

            prefetched = prepared_next;
        }

        eprintln!("[AI-WORKER] marking worker stopped");
        tracing::info!("ai_worker: marking worker stopped");
    });
}

// ---------------------------------------------------------------------------
// Stage A: Prepare — download images + create context cache
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum DownloadTag {
    Scheme(usize),
    Student(i32, usize),
}

async fn prepare(gemini: &GeminiClient, req: MarkRequest) -> PreparedRequest {
    let start = Instant::now();
    eprintln!(
        "[AI-PREPARE] starting (school={} exam={} scheme_images={} students={})",
        req.school,
        req.exam,
        req.scheme_get_urls.len(),
        req.students.len()
    );

    // Download ALL images concurrently (scheme + students)
    let mut download_handles = Vec::new();

    // Scheme images
    for (i, url) in req.scheme_get_urls.iter().enumerate() {
        let client = gemini.clone();
        let url = url.clone();
        download_handles.push(tokio::spawn(async move {
            let result = client.download_b64(&url).await;
            (DownloadTag::Scheme(i), result)
        }));
    }

    // Student images
    for (adm, urls) in &req.students {
        for (j, url) in urls.iter().enumerate() {
            let client = gemini.clone();
            let url = url.clone();
            let adm = *adm;
            download_handles.push(tokio::spawn(async move {
                let result = client.download_b64(&url).await;
                (DownloadTag::Student(adm, j), result)
            }));
        }
    }

    // Collect download results
    let mut scheme_b64: Vec<Option<String>> = vec![None; req.scheme_get_urls.len()];
    let mut student_map: std::collections::HashMap<i32, Vec<Option<String>>> =
        std::collections::HashMap::new();

    for (adm, urls) in &req.students {
        student_map.insert(*adm, vec![None; urls.len()]);
    }

    for handle in download_handles {
        match handle.await {
            Ok((tag, Ok(b64))) => match tag {
                DownloadTag::Scheme(i) => {
                    scheme_b64[i] = Some(b64);
                }
                DownloadTag::Student(adm, j) => {
                    if let Some(imgs) = student_map.get_mut(&adm) {
                        if j < imgs.len() {
                            imgs[j] = Some(b64);
                        }
                    }
                }
            },
            Ok((tag, Err(e))) => {
                eprintln!("[AI-PREPARE] download failed for {:?}: {}", tag, e);
                tracing::error!(tag = ?tag, error = %e, "ai_prepare: image download failed");
            }
            Err(e) => {
                eprintln!("[AI-PREPARE] download task panicked: {}", e);
                tracing::error!(error = %e, "ai_prepare: download task panicked");
            }
        }
    }

    // Build scheme parts from downloaded images
    let mut scheme_parts = Vec::with_capacity(req.scheme_get_urls.len() + 1);
    scheme_parts.push(serde_json::json!({
        "text": "## MARKING SCHEME\n\nThe following images contain the marking scheme for this paper. Study them carefully to identify every question, sub-question, mark allocation, expected answer, and any rubric notes (such as FT, Accept, OR, etc.). Determine the total marks for the paper by summing all mark allocations."
    }));

    for (i, maybe_b64) in scheme_b64.iter().enumerate() {
        if let Some(b64) = maybe_b64 {
            scheme_parts.push(serde_json::json!({
                "inline_data": { "mime_type": "image/jpeg", "data": b64 }
            }));
        } else {
            eprintln!(
                "[AI-PREPARE] WARNING: scheme image {} missing — skipping",
                i
            );
            tracing::warn!(index = i, "ai_prepare: scheme image missing");
        }
    }

    // Try to create context cache with retry (3 attempts)
    let cache_name = create_cache_with_retry(gemini, &scheme_parts).await;

    // Build student images list
    let student_images: Vec<(i32, Vec<String>)> = req
        .students
        .iter()
        .map(|(adm, _)| {
            let imgs = student_map
                .remove(adm)
                .unwrap_or_default()
                .into_iter()
                .flatten()
                .collect();
            (*adm, imgs)
        })
        .collect();

    eprintln!(
        "[AI-PREPARE] done in {}ms (cache={:?} students_with_images={})",
        start.elapsed().as_millis(),
        cache_name.as_deref().unwrap_or("NONE"),
        student_images.len()
    );
    tracing::info!(
        elapsed_ms = start.elapsed().as_millis(),
        cache_name = ?cache_name,
        student_count = student_images.len(),
        "ai_prepare: preparation complete"
    );

    PreparedRequest {
        mark_req: req,
        cache_name,
        student_images,
    }
}

/// Try to create a Gemini context cache with 3 retries.
async fn create_cache_with_retry(
    gemini: &GeminiClient,
    scheme_parts: &[serde_json::Value],
) -> Option<String> {
    let delays = [0u64, 2, 4];
    for (attempt, &delay_secs) in delays.iter().enumerate() {
        if delay_secs > 0 {
            tracing::warn!(
                attempt = attempt + 1,
                delay_secs = delay_secs,
                "retrying cache creation"
            );
            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
        }
        match gemini.create_context_cache(scheme_parts).await {
            Ok(name) => return Some(name),
            Err(e) => {
                eprintln!(
                    "[AI-PREPARE] cache creation attempt {} failed: {}",
                    attempt + 1,
                    e
                );
                tracing::warn!(
                    attempt = attempt + 1,
                    error = %e,
                    "cache creation attempt failed"
                );
            }
        }
    }
    eprintln!(
        "[AI-PREPARE] cache creation failed after 3 attempts — falling back to non-cached mode"
    );
    tracing::warn!("ai_prepare: cache creation failed after 3 retries — falling back");
    None
}

/// Retry a per-student marking call up to 3 times with exponential backoff.
async fn mark_student_with_retry(
    gemini: &GeminiClient,
    cache_name: &str,
    adm: i32,
    images: &[String],
) -> std::result::Result<crate::ai::gemini::StudentScore, Box<dyn std::error::Error + Send + Sync>>
{
    let delays = [0u64, 2, 4];
    let mut last_err = None;
    for (attempt, &delay_secs) in delays.iter().enumerate() {
        if delay_secs > 0 {
            tracing::warn!(
                adm = adm,
                attempt = attempt + 1,
                delay_secs = delay_secs,
                "retrying student marking"
            );
            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
        }
        match gemini.mark_student_cached(cache_name, adm, images).await {
            Ok(score) => return Ok(score),
            Err(e) => {
                tracing::warn!(
                    adm = adm,
                    attempt = attempt + 1,
                    error = %e,
                    "student marking attempt failed"
                );
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap())
}

// ---------------------------------------------------------------------------
// Stage B: Mark all students + write grades to DB
// ---------------------------------------------------------------------------

async fn mark_and_write(
    gemini: &GeminiClient,
    prepared: PreparedRequest,
) -> std::result::Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let task_start = Instant::now();
    let school = &prepared.mark_req.school;
    let exam = &prepared.mark_req.exam;
    let subject = prepared.mark_req.subject;
    let paper = prepared.mark_req.paper;
    let student_count = prepared.student_images.len();
    let cache_name = prepared.cache_name.as_deref();

    eprintln!(
        "[AI-MARK] marking {} students (school={} exam={} cached={})",
        student_count,
        school,
        exam,
        cache_name.is_some()
    );
    tracing::info!(
        school = %school,
        exam = %exam,
        student_count = student_count,
        cached = cache_name.is_some(),
        "ai_mark: starting student marking"
    );

    // If cache creation failed, fall back to non-cached mark_paper
    if cache_name.is_none() {
        eprintln!("[AI-MARK] no cache — falling back to non-cached mark_paper");
        tracing::info!("ai_mark: falling back to non-cached mark_paper");

        let scheme_urls = &prepared.mark_req.scheme_get_urls;
        let students = &prepared.mark_req.students;

        // Use the existing mark_paper which downloads images itself
        let scores = match gemini.mark_paper(scheme_urls, students).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[AI-MARK] fallback mark_paper failed: {}", e);
                tracing::error!(error = %e, "ai_mark: fallback mark_paper failed");
                return Err(e);
            }
        };

        let count = write_grades_to_db(school, exam, subject, paper, &scores);
        eprintln!(
            "[AI-MARK] fallback complete: {} grades in {}ms",
            count,
            task_start.elapsed().as_millis()
        );
        return Ok(count);
    }

    let cache_name_str = cache_name.unwrap(); // safe — checked above

    // Mark all students concurrently with bounded concurrency
    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT));
    let mut handles = Vec::with_capacity(student_count);

    for (adm, images) in &prepared.student_images {
        let client = gemini.clone();
        let sem = Arc::clone(&semaphore);
        let adm = *adm;
        let images = images.clone();
        let cn = cache_name_str.to_string();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore closed");
            mark_student_with_retry(&client, &cn, adm, &images).await
        });
        handles.push((adm, handle));
    }

    // Collect results
    let mut scores = Vec::with_capacity(student_count);
    let mut failed_adms = Vec::new();

    for (adm, handle) in handles {
        match handle.await {
            Ok(Ok(score)) => scores.push(score),
            Ok(Err(e)) => {
                eprintln!("[AI-MARK] ADM {} failed after retries: {}", adm, e);
                tracing::error!(adm = adm, error = %e, "ai_mark: student failed after retries");
                failed_adms.push(adm);
            }
            Err(e) => {
                eprintln!("[AI-MARK] ADM {} task panicked: {}", adm, e);
                tracing::error!(adm = adm, error = %e, "ai_mark: student task panicked");
                failed_adms.push(adm);
            }
        }
    }

    if !failed_adms.is_empty() {
        eprintln!(
            "[AI-MARK] {} of {} students failed: {:?}",
            failed_adms.len(),
            student_count,
            failed_adms
        );
        tracing::warn!(
            failed_count = failed_adms.len(),
            total_count = student_count,
            failed_adms = ?failed_adms,
            "ai_mark: some students failed"
        );
    }

    // Write whatever grades we have to DB
    let count = if !scores.is_empty() {
        write_grades_to_db(school, exam, subject, paper, &scores)
    } else {
        eprintln!("[AI-MARK] no scores to write");
        0
    };

    // Clean up: delete the Gemini cache (fire-and-forget)
    let gemini_cleanup = gemini.clone();
    let cn = cache_name_str.to_string();
    tokio::spawn(async move {
        gemini_cleanup.delete_context_cache(&cn).await;
    });

    eprintln!(
        "[AI-MARK] complete: {}/{} scored, {} grades written in {}ms",
        scores.len(),
        student_count,
        count,
        task_start.elapsed().as_millis()
    );
    tracing::info!(
        school = %school,
        exam = %exam,
        scored = scores.len(),
        total = student_count,
        grades_written = count,
        elapsed_ms = task_start.elapsed().as_millis(),
        "ai_mark: complete"
    );

    Ok(count)
}

// ---------------------------------------------------------------------------
// DB write helper (extracted from the old tokio::spawn block)
// ---------------------------------------------------------------------------

fn write_grades_to_db(
    school: &str,
    exam: &str,
    subject: i32,
    paper: Option<i32>,
    scores: &[crate::ai::gemini::StudentScore],
) -> usize {
    let write_start = Instant::now();

    // Fetch exam info for AI usage tracking (read-only, outside transaction)
    let exam_info = fetch_exam_year_term(school, exam);
    tracing::debug!(
        school = %school,
        exam = %exam,
        exam_info_ok = exam_info.is_ok(),
        "ai_write: exam year/term lookup for usage tracking"
    );

    tracing::debug!(
        school = %school,
        exam = %exam,
        student_count = scores.len(),
        "ai_write: opening DB transaction"
    );

    let ops_result: std::result::Result<Vec<(i32, u8)>, Error> = CONN.with(|cell| {
        let conn = &mut *cell.borrow_mut();
        conn.transaction(|conn| {
            let mut ops = Vec::with_capacity(scores.len());

            for score in scores {
                tracing::debug!(
                    adm = score.adm,
                    score = score.score,
                    total = score.total,
                    "ai_write: writing grade in transaction"
                );

                let grade = GradeInsert {
                    school: school.to_string(),
                    exam: exam.to_string(),
                    student: score.adm,
                    subject,
                    paper,
                    score: score.score as f32,
                    total: score.total,
                };

                let row_key = format!(
                    "{}|{}|{}|{}|{}",
                    school,
                    exam,
                    score.adm,
                    subject,
                    paper.map(|v| v.to_string()).unwrap_or_default()
                );

                let op = match insert::insert_grade(conn, &grade) {
                    Ok(_) => OP_INSERT,
                    Err(Error::Conflict) => {
                        tracing::debug!(adm = score.adm, "ai_write: grade already exists, updating");
                        let update_payload = UpdateGradePayload {
                            school: school.to_string(),
                            exam: exam.to_string(),
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
                            "ai_write: grade insert FAILED — rolling back"
                        );
                        return Err(e);
                    }
                };
                ops.push((score.adm, op));

                if let Ok((year, term)) = &exam_info {
                    let now = chrono::Utc::now().timestamp();
                    sql_query(
                        "INSERT INTO aiusage (school, student, year, term, allocated, used, created, updated) \
                         VALUES (?, ?, ?, ?, 0, 1, ?, ?) \
                         ON CONFLICT (school, student, year, term) \
                         DO UPDATE SET used = used + 1, updated = ?",
                    )
                    .bind::<Text, _>(school)
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
            eprintln!(
                "[AI-WRITE] DB transaction committed: {} grades in {}ms",
                ops.len(),
                write_start.elapsed().as_millis()
            );
            tracing::info!(
                school = %school,
                exam = %exam,
                written_count = ops.len(),
                elapsed_ms = write_start.elapsed().as_millis(),
                "ai_write: DB transaction committed — appending changelog"
            );

            let log_user = Id::system();
            LOG.with(|cell| {
                let mut log = cell.borrow_mut();
                for &(adm, op) in &ops {
                    let record = Record::new(log_user, TBL_GRADES, op, 0);
                    if let Err(e) = log.append(&record) {
                        tracing::error!(
                            adm = adm,
                            error = %e,
                            "ai_write: grades changelog append failed"
                        );
                    }
                    let ai_record = Record::new(log_user, TBL_AIUSAGE, OP_UPDATE, 0);
                    if let Err(e) = log.append(&ai_record) {
                        tracing::error!(
                            adm = adm,
                            error = %e,
                            "ai_write: aiusage changelog append failed"
                        );
                    }
                }
            });

            ops.len()
        }
        Err(e) => {
            eprintln!(
                "[AI-WRITE] DB transaction FAILED after {}ms: {}",
                write_start.elapsed().as_millis(),
                e
            );
            tracing::error!(
                school = %school,
                exam = %exam,
                error = %e,
                elapsed_ms = write_start.elapsed().as_millis(),
                "ai_write: DB transaction FAILED — no grades written"
            );
            0
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: look up exam year and term
// ---------------------------------------------------------------------------

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
