use diesel::sql_types::{Integer, Nullable, Text};
use std::sync::Arc;

use crate::ai::gemini::GeminiClient;
use crate::config::storage::sign;
use crate::db::changelog::{LOG, Record};
use crate::db::database::CONN;
use crate::db::database::tables::{papers as papers_db, question_bank};
use crate::proto::services::ai_marking::*;
use crate::types::error::{Error, Result};
use crate::types::id::Id;
use crate::types::token::Token;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::RunQueryDsl;

/// If `paper_id` is a legacy composite key (contains `|`), resolve it to
/// the new UUID paper ID by querying the papers table.  Otherwise return it
/// unchanged.
fn resolve_paper_id_if_legacy(paper_id: &str) -> String {
    if !paper_id.contains('|') {
        return paper_id.to_string();
    }
    let parts: Vec<&str> = paper_id.split('|').collect();
    if parts.len() < 6 {
        tracing::warn!(%paper_id, "unexpected legacy paper_id format");
        return paper_id.to_string();
    }
    let school = parts[0];
    let event = parts[1];
    let subject: i32 = parts[2].parse().unwrap_or(0);
    // paper number (parts[3]) and grade (parts[4]) and stream (parts[5]) are
    // available but we primarily match on (school, event, subject).

    let result = CONN.with(|conn| {
        #[derive(diesel::QueryableByName)]
        struct IdRow {
            #[diesel(sql_type = Text)]
            id: String,
        }

        // Strategy 1: exact event match
        if !event.is_empty() {
            let rows: Vec<IdRow> = sql_query(
                "SELECT id FROM papers WHERE school = ? AND event = ? AND subject = ? ORDER BY created",
            )
            .bind::<Text, _>(school)
            .bind::<Text, _>(event)
            .bind::<Integer, _>(subject)
            .load(conn)
            .unwrap_or_default();
            if let Some(row) = rows.into_iter().next() {
                return Some(row.id);
            }
        }

        // Strategy 2: event IS NULL
        {
            let rows: Vec<IdRow> = sql_query(
                "SELECT id FROM papers WHERE school = ? AND event IS NULL AND subject = ? ORDER BY created",
            )
            .bind::<Text, _>(school)
            .bind::<Integer, _>(subject)
            .load(conn)
            .unwrap_or_default();
            if let Some(row) = rows.into_iter().next() {
                return Some(row.id);
            }
        }

        // Strategy 3: via paper_schedules
        if !event.is_empty() {
            let rows: Vec<IdRow> = sql_query(
                "SELECT p.id FROM papers p \
                 JOIN paper_schedules ps ON ps.paper = p.id \
                 WHERE p.school = ? AND ps.event = ? AND p.subject = ? \
                 ORDER BY p.created",
            )
            .bind::<Text, _>(school)
            .bind::<Text, _>(event)
            .bind::<Integer, _>(subject)
            .load(conn)
            .unwrap_or_default();
            if let Some(row) = rows.into_iter().next() {
                return Some(row.id);
            }
        }

        None
    });

    match result {
        Some(id) => {
            tracing::info!(legacy = %paper_id, resolved = %id, "resolved legacy paper_id");
            id
        }
        None => {
            tracing::warn!(%paper_id, "could not resolve legacy paper_id to UUID, using as-is");
            paper_id.to_string()
        }
    }
}
use std::time::Instant;
use tokio::sync::mpsc;

const TBL_AIUSAGE: u8 = 24;
const OP_UPDATE: u8 = 1;

/// Max concurrent AI API requests when marking students within one paper.
const MAX_CONCURRENT: usize = 4;

/// Queue capacity for pending mark requests.
const QUEUE_CAPACITY: usize = 32;

// ---------------------------------------------------------------------------
// Request types for the channel queue
// ---------------------------------------------------------------------------

struct MarkRequest {
    paper_id: String,
    school: String, // kept for batch display name + logging
    
    students: Vec<(i32, Vec<String>)>, // (adm, [S3 GET URLs])
}

struct PreparedRequest {
    mark_req: MarkRequest,
    
    student_images: Vec<(i32, Vec<String>)>, // (adm, [base64 images])
}

#[derive(QueryableByName)]
struct PaperMeta {
    #[diesel(sql_type = Text)]
    school: String,
    #[diesel(sql_type = Nullable<Text>)]
    event: Option<String>,
    #[diesel(sql_type = Integer)]
    subject: i32,
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
        resume_incomplete_jobs(&tx);
        AiMarkingServer::new(Self { config, tx })
    }

    async fn request_upload_urls(
        &self,
        _token: Token,
        req: UploadUrlsRequest,
    ) -> Result<UploadUrlsResponse> {
        // Resolve legacy composite-format paper IDs (school|event|subject|paper|grade|stream)
        // to the new UUID paper ID.  Old clients may still pass the composite format.
        let paper_id = resolve_paper_id_if_legacy(&req.paper_id);

        tracing::info!(
            paper_id = %paper_id,

            student_count = req.students.len(),
            "request_upload_urls: generating presigned PUT URLs"
        );
        let scheme_urls: Vec<SignedUrl> = Vec::new();

        let student_urls: Vec<StudentSignedUrls> = req
            .students
            .iter()
            .map(|s| {
                let urls: Vec<SignedUrl> = (0..s.count)
                    .map(|i| {
                        let key = format!("papers/{}/answers/{}/page_{}.jpg", paper_id, s.adm, i);
                        let url = sign::url(&key, sign::PUT_TTL, true);
                        SignedUrl { key, url }
                    })
                    .collect();
                StudentSignedUrls { adm: s.adm, urls }
            })
            .collect();

        const TBL_ANSWER_PAGES: u8 = 37;
        const OP_INSERT: u8 = 0;
        const OP_DELETE: u8 = 2;

        // Record scheme + answer page keys in DB.
        // Delete old answer pages per student first so replaced sheets overwrite
        // rather than accumulate stale rows.  Also write changelog entries so
        // other clients learn about the deletions and insertions via watch.
        CONN.with(|conn| {
            // Look up paper metadata for changelog row_keys.
            let meta: Option<PaperMeta> = sql_query(
                "SELECT school, event, subject FROM papers WHERE id = ?",
            )
            .bind::<Text, _>(&paper_id)
            .load(conn)
            .ok()
            .and_then(|rows: Vec<PaperMeta>| rows.into_iter().next());
            let school = meta.as_ref().map(|m| m.school.clone()).unwrap_or_default();
            let exam = meta.as_ref().and_then(|m| m.event.clone()).unwrap_or_default();
            let subject = meta.map(|m| m.subject).unwrap_or(0);
            let log_user = Id::system();

            for (i, su) in scheme_urls.iter().enumerate() {
                let _ = question_bank::insert_scheme_page(conn, &paper_id, i as i16, &su.key);
            }
            for stu in &req.students {
                // Query existing pages so we can log their deletion.
                let old_pages: Vec<i16> =
                    question_bank::get_answer_pages(conn, &paper_id, stu.adm)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(page, _)| page)
                        .collect();

                // Delete old pages from DB.
                let _ = question_bank::delete_answer_pages_for_student(
                    conn, &paper_id, stu.adm,
                );

                // Log changelog deletes for each old page.
                for page in &old_pages {
                    let row_key = format!(
                        "{}|{}|{}|{}||{}",
                        school, exam, stu.adm, subject, page
                    );
                    let _ = LOG.with(|cell| {
                        cell.borrow_mut().append(
                            &Record::new(log_user, TBL_ANSWER_PAGES, OP_DELETE, 0)
                        )
                    });
                    let _ = LOG.with(|cell| {
                        cell.borrow_mut().append_delete(TBL_ANSWER_PAGES, &row_key)
                    });
                }

                // Insert new pages with current S3 keys.
                let urls = student_urls.iter().find(|x| x.adm == stu.adm);
                if let Some(su) = urls {
                    for (i, signed) in su.urls.iter().enumerate() {
                        let _ = question_bank::insert_answer_page(
                            conn, &paper_id, stu.adm, i as i16, &signed.key,
                        );
                        // Log changelog insert for this page.
                        let _ = LOG.with(|cell| {
                            cell.borrow_mut().append(
                                &Record::new(log_user, TBL_ANSWER_PAGES, OP_INSERT, 0)
                            )
                        });
                    }
                }
            }
        });

        tracing::info!(
            paper_id = %paper_id,
            scheme_url_count = scheme_urls.len(),
            student_url_count = student_urls.len(),
            "request_upload_urls: done"
        );

        Ok(UploadUrlsResponse {
            
            student_urls,
        })
    }

    async fn mark_paper(&self, _token: Token, req: MarkPaperRequest) -> Result<MarkPaperResponse> {
        // Resolve legacy composite-format paper IDs (school|event|subject|paper|grade|stream)
        // to the new UUID paper ID — same as request_upload_urls does.
        let paper_id = resolve_paper_id_if_legacy(&req.paper_id);
        let student_count = req.students.len();

        tracing::info!(
            paper_id = %paper_id,
            total_marks = req.total_marks,
            
            student_count = student_count,
            "mark_paper: RPC received"
        );

        // Look up school for batch display name
        let school = CONN.with(|conn| {
            papers_db::get_paper(conn, &paper_id)
                .ok()
                .flatten()
                .map(|p| p.school)
                .unwrap_or_default()
        });

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
            paper_id: paper_id.clone(),
            school: school.clone(),

            students,
        };

        self.tx.try_send(mark_req).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => {
                tracing::warn!(paper_id = %paper_id, "mark_paper: queue full — rejecting");
                Error::SlowDown
            }
            mpsc::error::TrySendError::Closed(_) => {
                tracing::error!("mark_paper: worker channel closed");
                Error::Internal("AI marking worker channel closed — try restarting the server".into())
            }
        })?;

        tracing::info!(
            paper_id = %paper_id,
            school = %school,
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
// Background marking worker
// ---------------------------------------------------------------------------

fn spawn_marking_worker(rx: mpsc::Receiver<MarkRequest>, gemini: GeminiClient) {
    tokio::spawn(async move {
        let mut rx = rx;
        let mut prefetched: Option<PreparedRequest> = None;

        tracing::info!("ai_worker: marking worker started");

        loop {
            let current = if let Some(prepared) = prefetched.take() {
                tracing::info!(
                    paper_id = %prepared.mark_req.paper_id,
                    "ai_worker: using prefetched request"
                );
                prepared
            } else {
                let req = match rx.recv().await {
                    Some(r) => r,
                    None => {
                        tracing::info!("ai_worker: channel closed — shutting down");
                        break;
                    }
                };
                tracing::info!(
                    paper_id = %req.paper_id,
                    school = %req.school,
                    student_count = req.students.len(),
                    "ai_worker: received request — preparing"
                );
                prepare(&gemini, req).await
            };

            let maybe_next = rx.try_recv().ok();
            let has_next = maybe_next.is_some();
            if has_next {
                tracing::info!("ai_worker: prefetching next request in parallel");
            }

            let gemini_for_prefetch = gemini.clone();
            let (mark_result, prepared_next) =
                tokio::join!(route_marking(&gemini, current), async {
                    match maybe_next {
                        Some(next_req) => Some(prepare(&gemini_for_prefetch, next_req).await),
                        None => None,
                    }
                });

            match mark_result {
                Ok(count) => tracing::info!(grades_written = count, "ai_worker: marking complete"),
                Err(e) => tracing::error!(error = %e, "ai_worker: marking failed"),
            }

            prefetched = prepared_next;
        }

        tracing::info!("ai_worker: marking worker stopped");
    });
}

/// Scan marking_queue for jobs left incomplete by a previous server shutdown
/// and re-queue them for the worker to pick up.
fn resume_incomplete_jobs(tx: &mpsc::Sender<MarkRequest>) {
    #[derive(diesel::QueryableByName)]
    struct IncompleteJob {
        #[diesel(sql_type = Text)]
        paper: String,
    }

    #[derive(diesel::QueryableByName)]
    struct AnswerPageRow {
        #[diesel(sql_type = Integer)]
        student: i32,
        #[diesel(sql_type = Text)]
        key: String,
    }

    let requests: Vec<MarkRequest> = CONN.with(|conn| {
        let jobs: Vec<IncompleteJob> = match sql_query(
            "SELECT paper FROM marking_queue WHERE phase >= 1 AND phase <= 4",
        )
        .load(conn)
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!(error = %e, "ai_resume: failed to query incomplete jobs");
                return Vec::new();
            }
        };

        if jobs.is_empty() {
            return Vec::new();
        }

        tracing::info!(count = jobs.len(), "ai_resume: found incomplete marking jobs");

        let mut out = Vec::with_capacity(jobs.len());

        for job in &jobs {
            let paper_id = &job.paper;

            // Reset to phase 1 so old phase doesn't mislead.
            let _ = question_bank::update_marking_status(
                conn,
                paper_id,
                1,
                "Resuming...",
                None,
                0,
                0,
            );

            let school = papers_db::get_paper(conn, paper_id)
                .ok()
                .flatten()
                .map(|p| p.school)
                .unwrap_or_default();

            let pages: Vec<AnswerPageRow> = match sql_query(
                "SELECT student, key FROM answer_pages \
                 WHERE paper = ? ORDER BY student, page",
            )
            .bind::<Text, _>(paper_id)
            .load(conn)
            {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::warn!(paper_id = %paper_id, error = %e,
                        "ai_resume: failed to query answer pages");
                    let _ = question_bank::update_marking_status(
                        conn, paper_id, 6, "Failed",
                        Some("Failed to load answer pages for resumption"),
                        0, 0,
                    );
                    continue;
                }
            };

            if pages.is_empty() {
                tracing::warn!(paper_id = %paper_id,
                    "ai_resume: no answer pages found — marking as failed");
                let _ = question_bank::update_marking_status(
                    conn, paper_id, 6, "No answer pages",
                    Some("No answer pages found for this paper"),
                    0, 0,
                );
                continue;
            }

            // Group keys by student, preserving page order.
            let mut student_map: std::collections::BTreeMap<i32, Vec<String>> =
                std::collections::BTreeMap::new();
            for page in &pages {
                let url = sign::url(&page.key, sign::GET_TTL, false);
                student_map.entry(page.student).or_default().push(url);
            }

            let students: Vec<(i32, Vec<String>)> = student_map.into_iter().collect();
            let student_count = students.len();

            tracing::info!(
                paper_id = %paper_id,
                student_count,
                "ai_resume: re-queuing marking job"
            );

            out.push(MarkRequest {
                paper_id: paper_id.clone(),
                school,
                students,
            });
        }

        out
    });

    for req in requests {
        match tx.try_send(req) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(req)) => {
                tracing::warn!(
                    paper_id = %req.paper_id,
                    "ai_resume: queue full — dropping resumed job"
                );
                // Mark as failed so the user can re-trigger manually.
                CONN.with(|conn| {
                    let _ = question_bank::update_marking_status(
                        conn, &req.paper_id, 6, "Queue full",
                        Some("Marking queue is full — please try again"),
                        0, 0,
                    );
                });
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                tracing::error!("ai_resume: channel closed — aborting resume");
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Stage A: Prepare — download images + create context cache
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum DownloadTag {
    Student(i32, usize),
}

async fn prepare(gemini: &GeminiClient, req: MarkRequest) -> PreparedRequest {
    let mut student_map: std::collections::HashMap<i32, Vec<Option<String>>> = std::collections::HashMap::new();
    
    let client = gemini.clone();
    let mut join_set = tokio::task::JoinSet::new();
    
    for (adm, urls) in &req.students {
        student_map.entry(*adm).or_insert_with(|| vec![None; urls.len()]);
        for (j, url) in urls.iter().enumerate() {
            let url = url.clone();
            let client = client.clone();
            let adm = *adm;
            join_set.spawn(async move {
                let result = client.download_b64(&url).await;
                (DownloadTag::Student(adm, j), result)
            });
        }
    }
    
    while let Some(res) = join_set.join_next().await {
        match res {
            Ok((tag, Ok(b64))) => match tag {
                DownloadTag::Student(adm, j) => {
                    if let Some(list) = student_map.get_mut(&adm) {
                        list[j] = Some(b64);
                    }
                }
            },
            Ok((tag, Err(e))) => {
                tracing::error!(tag = ?tag, error = %e, "ai_prepare: image download failed");
            }
            Err(e) => {
                tracing::error!(error = %e, "ai_prepare: download task panicked");
            }
        }
    }

    let student_images: Vec<(i32, Vec<String>)> = req.students.iter().map(|(adm, _)| {
        let imgs = student_map.remove(adm).unwrap_or_default().into_iter().flatten().collect();
        (*adm, imgs)
    }).collect();

    tracing::info!(paper_id = %req.paper_id, student_count = student_images.len(), "ai_prepare: complete");

    PreparedRequest {
        mark_req: req,
        student_images,
    }
}

// ---------------------------------------------------------------------------
// Retry helpers
// ---------------------------------------------------------------------------

async fn create_cache_with_retry(
    gemini: &GeminiClient,
    scheme_parts: &[serde_json::Value],
) -> Option<String> {
    let delays = [0u64, 2, 4];
    for (attempt, &delay_secs) in delays.iter().enumerate() {
        if delay_secs > 0 {
            tracing::warn!(attempt = attempt + 1, delay_secs, "retrying cache creation");
            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
        }
        match gemini.create_context_cache(scheme_parts).await {
            Ok(name) => return Some(name),
            Err(e) => {
                tracing::warn!(attempt = attempt + 1, error = %e, "cache creation attempt failed");
            }
        }
    }
    tracing::warn!("ai_prepare: cache creation failed after 3 retries — falling back");
    None
}

async fn mark_student_with_retry(
    gemini: &GeminiClient,
    adm: i32,
    images: &[String],
) -> std::result::Result<crate::ai::gemini::StudentScore, Box<dyn std::error::Error + Send + Sync>>
{
    let delays = [0u64, 2, 4];
    let mut last_err = None;
    for (attempt, &delay_secs) in delays.iter().enumerate() {
        if delay_secs > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
        }
        match gemini.mark_student_uncached(adm, images).await {
            Ok(score) => return Ok(score),
            Err(e) => {
                tracing::warn!(adm, attempt = attempt + 1, error = %e, "student marking attempt failed");
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap())
}

async fn mark_student_cached_with_retry(
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
            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
        }
        match gemini.mark_student_cached(cache_name, adm, images).await {
            Ok(score) => return Ok(score),
            Err(e) => {
                tracing::warn!(adm, attempt = attempt + 1, error = %e, "student cached marking attempt failed");
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap())
}

// ---------------------------------------------------------------------------
// Stage B helpers: real-time concurrent + batch API
// ---------------------------------------------------------------------------

async fn mark_students_realtime(
    gemini: &GeminiClient,
    student_images: &[(i32, Vec<String>)],
) -> (Vec<crate::ai::gemini::StudentScore>, Vec<i32>) {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT));
    let mut handles = Vec::with_capacity(student_images.len());

    for (adm, images) in student_images {
        let client = gemini.clone();
        let sem = Arc::clone(&semaphore);
        let adm = *adm;
        let images = images.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore closed");
            mark_student_with_retry(&client, adm, &images).await
        });
        handles.push((adm, handle));
    }

    let mut scores = Vec::with_capacity(student_images.len());
    let mut failed_adms = Vec::new();

    for (adm, handle) in handles {
        match handle.await {
            Ok(Ok(score)) => scores.push(score),
            Ok(Err(e)) => {
                tracing::error!(adm, error = %e, "ai_mark: student failed after retries");
                failed_adms.push(adm);
            }
            Err(e) => {
                tracing::error!(adm, error = %e, "ai_mark: student task panicked");
                failed_adms.push(adm);
            }
        }
    }

    (scores, failed_adms)
}

async fn mark_students_realtime_cached(
    gemini: &GeminiClient,
    cache_name: &str,
    student_images: &[(i32, Vec<String>)],
) -> (Vec<crate::ai::gemini::StudentScore>, Vec<i32>) {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT));
    let mut handles = Vec::with_capacity(student_images.len());

    for (adm, images) in student_images {
        let client = gemini.clone();
        let sem = Arc::clone(&semaphore);
        let adm = *adm;
        let images = images.clone();
        let cn = cache_name.to_string();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore closed");
            mark_student_cached_with_retry(&client, &cn, adm, &images).await
        });
        handles.push((adm, handle));
    }

    let mut scores = Vec::with_capacity(student_images.len());
    let mut failed_adms = Vec::new();

    for (adm, handle) in handles {
        match handle.await {
            Ok(Ok(score)) => scores.push(score),
            Ok(Err(e)) => {
                tracing::error!(adm, error = %e, "ai_mark: student failed after retries");
                failed_adms.push(adm);
            }
            Err(e) => {
                tracing::error!(adm, error = %e, "ai_mark: student task panicked");
                failed_adms.push(adm);
            }
        }
    }

    (scores, failed_adms)
}

async fn mark_batch_with_fallback(
    gemini: &GeminiClient,
    
    student_images: &[(i32, Vec<String>)],
    paper_id: &str,
) -> Vec<crate::ai::gemini::StudentScore> {
    let display_name = format!("marking-{}", paper_id);

    let students_ref: Vec<(i32, &[String])> = student_images
        .iter()
        .map(|(adm, imgs)| (*adm, imgs.as_slice()))
        .collect();

    let batch_name = match gemini
        .create_batch_job(&students_ref, &display_name)
        .await
    {
        Ok(name) => name,
        Err(e) => {
            tracing::warn!(error = %e, "ai_batch: batch creation failed — falling back to real-time");
            return Vec::new();
        }
    };

    tracing::info!(batch_name = %batch_name, "ai_batch: polling started");

    let mut scores = Vec::new();
    let max_polls: usize = 120;

    for poll in 0..max_polls {
        tokio::time::sleep(std::time::Duration::from_secs(15)).await;

        match gemini.get_batch_status(&batch_name).await {
            Ok(status) => {
                use crate::ai::gemini::{BatchStatus, BatchStudentResult};
                match status {
                    BatchStatus::Pending | BatchStatus::Running => {
                        if poll % 4 == 0 {
                            tracing::info!(batch_name = %batch_name, poll = poll + 1, "ai_batch: waiting");
                        }
                    }
                    BatchStatus::Succeeded(results) => {
                        for result in results {
                            match result {
                                BatchStudentResult::Ok(score) => {
                                    tracing::info!(
                                        adm = score.adm,
                                        score = score.score,
                                        "ai_batch: student scored"
                                    );
                                    scores.push(score);
                                }
                                BatchStudentResult::Err { adm_key, error } => {
                                    tracing::warn!(adm_key = %adm_key, error = %error, "ai_batch: student failed");
                                }
                            }
                        }
                        break;
                    }
                    BatchStatus::Failed(msg) => {
                        tracing::error!(batch_name = %batch_name, error = %msg, "ai_batch: failed");
                        break;
                    }
                    BatchStatus::Cancelled | BatchStatus::Expired => {
                        tracing::warn!(batch_name = %batch_name, "ai_batch: cancelled/expired");
                        break;
                    }
                }
            }
            Err(e) => {
                tracing::warn!(batch_name = %batch_name, error = %e, poll = poll + 1, "ai_batch: poll failed");
            }
        }
    }

    if scores.is_empty() {
        tracing::warn!(batch_name = %batch_name, "ai_batch: timed out — cancelling");
        gemini.cancel_batch_job(&batch_name).await;
    }

    scores
}

// ---------------------------------------------------------------------------
// Route marking: per-question (question bank) or legacy (whole-paper)
// ---------------------------------------------------------------------------

async fn route_marking(
    gemini: &GeminiClient,
    prepared: PreparedRequest,
) -> std::result::Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let paper_id = prepared.mark_req.paper_id.clone();
    let has_paper_questions = CONN.with(|conn| {
        match question_bank::get_paper_questions(conn, &paper_id, None) {
            Ok(pqs) => !pqs.is_empty(),
            Err(_) => false,
        }
    });

    let result = if has_paper_questions {
        tracing::info!(
            paper_id = %paper_id,
            "ai_worker: routing to per-student cached marking"
        );
        mark_and_write_cached(gemini, prepared).await
    } else {
        mark_and_write(gemini, prepared).await
    };

    if let Err(e) = &result {
        let msg = e.to_string();
        CONN.with(|conn| {
            let _ = question_bank::update_marking_status(
                conn, &paper_id, 6, "Failed", Some(&msg), 0, 0,
            );
        });
    }

    result
}

// ---------------------------------------------------------------------------
// Stage B: Per-student cached marking (papers with structured questions)
// ---------------------------------------------------------------------------

async fn mark_and_write_cached(
    gemini: &GeminiClient,
    prepared: PreparedRequest,
) -> std::result::Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let task_start = Instant::now();
    let paper_id = prepared.mark_req.paper_id.clone();
    let total_students = prepared.student_images.len();

    tracing::info!(
        paper_id = %paper_id,
        student_count = total_students,
        "ai_cached: starting per-student cached marking"
    );

    // Init marking queue + phase 1
    CONN.with(|conn| {
        let _ = question_bank::upsert_marking_queue(conn, &paper_id);
        let _ = question_bank::update_marking_status(
            conn,
            &paper_id,
            1,
            "Loading questions...",
            None,
            total_students as i32,
            0,
        );
    });

    // 1. Load paper questions
    let paper_qs = match CONN.with(|conn| {
        question_bank::get_paper_questions(conn, &paper_id, None)
    }) {
        Ok(pqs) => pqs,
        Err(e) => return Err(format!("Failed to load paper questions: {:?}", e).into()),
    };

    if paper_qs.is_empty() {
        return Err("No paper_questions found for cached marking".into());
    }

    // 2. Load question data: (q_num, q_text, marks, rubric_text, images_b64)
    struct QData {
        q_num: i32,
        text: String,
        marks: i16,
        rubric: String,
        images_b64: Vec<(String, Option<String>)>,
    }

    let mut questions_data: Vec<QData> = Vec::with_capacity(paper_qs.len());

    for (idx, pq) in paper_qs.iter().enumerate() {
        let q = match CONN.with(|conn| question_bank::get_question(conn, pq.question)) {
            Ok(Some(q)) => q,
            Ok(None) => {
                tracing::warn!(question_id = pq.question, "ai_cached: question not found — skipping");
                continue;
            }
            Err(e) => {
                tracing::warn!(question_id = pq.question, error = ?e, "ai_cached: failed to load question");
                continue;
            }
        };

        let rubric = match CONN.with(|conn| question_bank::get_rubric_criteria(conn, pq.question)) {
            Ok(r) => r,
            Err(_) => Vec::new(),
        };

        let rubric_text: String = rubric
            .iter()
            .enumerate()
            .map(|(j, r)| format!("{}. {} ({} marks)", j + 1, r.criterion, r.marks))
            .collect::<Vec<_>>()
            .join("\n");

        let images = match CONN.with(|conn| question_bank::get_question_images(conn, pq.question)) {
            Ok(imgs) => imgs,
            Err(_) => Vec::new(),
        };

        let mut imgs_b64: Vec<(String, Option<String>)> = Vec::new();
        for img_row in &images {
            let get_url = sign::url(&img_row.key, sign::GET_TTL, false);
            match gemini.download_b64(&get_url).await {
                Ok(b64) => imgs_b64.push((b64, img_row.caption.clone())),
                Err(e) => {
                    tracing::warn!(
                        question = pq.question,
                        key = %img_row.key,
                        error = %e,
                        "ai_cached: question image download failed — skipping image"
                    );
                }
            }
        }

        questions_data.push(QData {
            q_num: (idx + 1) as i32,
            text: q.body.clone(),
            marks: q.marks,
            rubric: rubric_text,
            images_b64: imgs_b64,
        });
    }

    if questions_data.is_empty() {
        return Err("No valid question data loaded for cached marking".into());
    }

    tracing::info!(
        question_count = questions_data.len(),
        "ai_cached: question data loaded"
    );

    // Phase 2: creating cache
    CONN.with(|conn| {
        let _ = question_bank::update_marking_status(
            conn,
            &paper_id,
            2,
            "Creating cache...",
            None,
            total_students as i32,
            0,
        );
    });

    // 3. Build cache parts from questions + rubrics
    let cache_input: Vec<(i32, &str, i16, &str, &[(String, Option<String>)])> = questions_data
        .iter()
        .map(|qd| {
            (qd.q_num, qd.text.as_str(), qd.marks, qd.rubric.as_str(), qd.images_b64.as_slice())
        })
        .collect();

    let scheme_parts = GeminiClient::build_question_cache_parts(&cache_input);

    // 4. Create context cache — fall back to uncached marking if unsupported.
    let maybe_cache = create_cache_with_retry(gemini, &scheme_parts).await;

    // 5. Mark all students (cached if possible, otherwise uncached).
    CONN.with(|conn| {
        let _ = question_bank::update_marking_status(
            conn,
            &paper_id,
            3,
            &format!("Marking {total_students} students..."),
            None,
            total_students as i32,
            0,
        );
    });

    let (scores, failed_adms) = if let Some(ref cache_name) = maybe_cache {
        let result = mark_students_realtime_cached(gemini, cache_name, &prepared.student_images).await;
        gemini.delete_context_cache(cache_name).await;
        result
    } else {
        tracing::warn!(paper_id = %paper_id, "ai_cached: cache unavailable — falling back to uncached marking");
        mark_students_realtime(gemini, &prepared.student_images).await
    };

    if !failed_adms.is_empty() {
        tracing::warn!(
            failed_count = failed_adms.len(),
            total_count = total_students,
            "ai_cached: some students failed"
        );
    }

    // 7. Write grades + token usage to DB
    // Phase 4: writing grades
    CONN.with(|conn| {
        let _ = question_bank::update_marking_status(
            conn,
            &paper_id,
            4,
            "Writing grades...",
            None,
            total_students as i32,
            scores.len() as i32,
        );
    });

    let count = if !scores.is_empty() {
        write_grades_to_db(&paper_id, &scores)
    } else {
        0
    };

    // Phase 5: complete
    CONN.with(|conn| {
        let _ = question_bank::update_marking_status(
            conn,
            &paper_id,
            5,
            "Complete",
            None,
            total_students as i32,
            count as i32,
        );
    });

    tracing::info!(
        paper_id = %paper_id,
        scored = scores.len(),
        total = total_students,
        grades_written = count,
        elapsed_ms = task_start.elapsed().as_millis(),
        "ai_cached: complete"
    );

    Ok(count)
}

// ---------------------------------------------------------------------------
// Stage B: Mark all students + write grades (legacy whole-paper flow)
// ---------------------------------------------------------------------------

async fn mark_and_write(
    gemini: &GeminiClient,
    prepared: PreparedRequest,
) -> std::result::Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let task_start = Instant::now();
    let paper_id = &prepared.mark_req.paper_id;
    let student_count = prepared.student_images.len();

    tracing::info!(
        paper_id = %paper_id,
        student_count,
        cached = false,
        "ai_mark: starting student marking"
    );

    // Init marking queue + phase 1
    CONN.with(|conn| {
        let _ = question_bank::upsert_marking_queue(conn, paper_id);
        let _ = question_bank::update_marking_status(
            conn,
            paper_id,
            1,
            "Downloading answer sheets...",
            None,
            student_count as i32,
            0,
        );
    });

    // Phase 3: marking
    CONN.with(|conn| {
        let _ = question_bank::update_marking_status(
            conn,
            paper_id,
            3,
            "Marking students...",
            None,
            student_count as i32,
            0,
        );
    });

    let (scores, failed_adms) = if student_count > 1 {
        tracing::info!(student_count, "ai_mark: using batch API");
        let batch_scores =
            mark_batch_with_fallback(gemini, &prepared.student_images, paper_id)
                .await;

        if !batch_scores.is_empty() {
            (batch_scores, Vec::new())
        } else {
            tracing::warn!("ai_mark: batch failed — falling back to real-time");
            mark_students_realtime(gemini, &prepared.student_images).await
        }
    } else {
        mark_students_realtime(gemini, &prepared.student_images).await
    };

    if !failed_adms.is_empty() {
        tracing::warn!(
            failed_count = failed_adms.len(),
            total_count = student_count,
            "ai_mark: some students failed"
        );
    }

    // Phase 4: writing grades
    CONN.with(|conn| {
        let _ = question_bank::update_marking_status(
            conn,
            paper_id,
            4,
            "Writing grades...",
            None,
            student_count as i32,
            scores.len() as i32,
        );
    });

    let count = if !scores.is_empty() {
        write_grades_to_db(paper_id, &scores)
    } else {
        0
    };

    // Phase 5: complete
    CONN.with(|conn| {
        let _ = question_bank::update_marking_status(
            conn,
            paper_id,
            5,
            "Complete",
            None,
            student_count as i32,
            count as i32,
        );
    });

    tracing::info!(
        paper_id = %paper_id,
        scored = scores.len(),
        total = student_count,
        grades_written = count,
        elapsed_ms = task_start.elapsed().as_millis(),
        "ai_mark: complete"
    );

    Ok(count)
}

// ---------------------------------------------------------------------------
// DB write helper
// ---------------------------------------------------------------------------

fn write_grades_to_db(paper_id: &str, scores: &[crate::ai::gemini::StudentScore]) -> usize {
    let write_start = Instant::now();
    let mut written = 0usize;

    for score in scores {
        let result = CONN.with(|conn| {
            papers_db::upsert_grade(conn, paper_id, score.adm, score.score as f32)
        });

        match result {
            Ok(()) => {
                written += 1;

                // Store token usage if available
                if let Some(ref usage) = score.usage {
                    let _ = CONN.with(|conn| {
                        use crate::db::database::tables::insert::insert_ai_token_log;
                        insert_ai_token_log(conn, paper_id, score.adm, usage)
                    });
                }

                // Append changelog record for grade
                let log_user = Id::system();
                LOG.with(|cell| {
                    let mut log = cell.borrow_mut();
                    let record = Record::new(log_user, 18u8, 0u8, 0);
                    if let Err(e) = log.append(&record) {
                        tracing::error!(adm = score.adm, error = %e, "ai_write: grades changelog append failed");
                    }
                    let ai_record = Record::new(log_user, TBL_AIUSAGE, OP_UPDATE, 0);
                    if let Err(e) = log.append(&ai_record) {
                        tracing::error!(adm = score.adm, error = %e, "ai_write: aiusage changelog append failed");
                    }
                });
            }
            Err(e) => {
                tracing::error!(
                    paper_id = %paper_id,
                    adm = score.adm,
                    error = %e,
                    "ai_write: grade upsert FAILED"
                );
            }
        }
    }

    tracing::info!(
        paper_id = %paper_id,
        written_count = written,
        elapsed_ms = write_start.elapsed().as_millis(),
        "ai_write: grades written"
    );

    written
}
