use crate::ai::gemini::{AllQuestionInput, GeminiClient, QuestionScore};
use crate::config::storage::sign;
use crate::db::changelog::{LOG, Record};
use crate::db::database::CONN;
use crate::db::database::tables::{papers as papers_db, question_bank};
use crate::proto::services::ai_marking::*;
use crate::types::error::{Error, Result};
use crate::types::id::Id;
use crate::types::token::Token;
use std::sync::Arc;
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
    scheme_get_urls: Vec<String>,
    students: Vec<(i32, Vec<String>)>, // (adm, [S3 GET URLs])
}

struct PreparedRequest {
    mark_req: MarkRequest,
    cache_name: Option<String>,
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
        let paper_id = &req.paper_id;

        tracing::info!(
            paper_id = %paper_id,
            scheme_count = req.scheme_count,
            student_count = req.students.len(),
            "request_upload_urls: generating presigned PUT URLs"
        );

        let scheme_urls: Vec<SignedUrl> = (0..req.scheme_count)
            .map(|i| {
                let key = format!("papers/{}/scheme/page_{}.jpg", paper_id, i);
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
                        let key = format!("papers/{}/answers/{}/page_{}.jpg", paper_id, s.adm, i);
                        let url = sign::url(&key, sign::PUT_TTL, true);
                        SignedUrl { key, url }
                    })
                    .collect();
                StudentSignedUrls { adm: s.adm, urls }
            })
            .collect();

        // Record scheme + answer page keys in DB
        CONN.with(|conn| {
            for (i, su) in scheme_urls.iter().enumerate() {
                let _ = question_bank::insert_scheme_page(conn, paper_id, i as i16, &su.key);
            }
            for stu in &req.students {
                for (i, su) in student_urls
                    .iter()
                    .find(|x| x.adm == stu.adm)
                    .map(|x| x.urls.iter().enumerate())
                    .into_iter()
                    .flatten()
                {
                    let _ = question_bank::insert_answer_page(
                        conn, paper_id, stu.adm, i as i16, &su.key,
                    );
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
            scheme_urls,
            student_urls,
        })
    }

    async fn mark_paper(&self, _token: Token, req: MarkPaperRequest) -> Result<MarkPaperResponse> {
        let paper_id = req.paper_id.clone();
        let student_count = req.students.len();

        tracing::info!(
            paper_id = %paper_id,
            total_marks = req.total_marks,
            scheme_key_count = req.scheme_keys.len(),
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
            paper_id: paper_id.clone(),
            school: school.clone(),
            scheme_get_urls,
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
    tracing::info!(
        paper_id = %req.paper_id,
        scheme_images = req.scheme_get_urls.len(),
        students = req.students.len(),
        "ai_prepare: starting"
    );

    let mut download_handles = Vec::new();

    for (i, url) in req.scheme_get_urls.iter().enumerate() {
        let client = gemini.clone();
        let url = url.clone();
        download_handles.push(tokio::spawn(async move {
            let result = client.download_b64(&url).await;
            (DownloadTag::Scheme(i), result)
        }));
    }

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
                tracing::error!(tag = ?tag, error = %e, "ai_prepare: image download failed");
            }
            Err(e) => {
                tracing::error!(error = %e, "ai_prepare: download task panicked");
            }
        }
    }

    let mut scheme_parts = Vec::with_capacity(req.scheme_get_urls.len() + 1);
    scheme_parts.push(serde_json::json!({
        "text": "## MARKING SCHEME\n\nThe following images contain the marking scheme for this paper. Study them carefully to identify every question, sub-question, mark allocation, rubric criterion, expected answer, and any rubric notes (such as FT, Accept, OR, etc.). The rubric criteria are your PRIMARY scoring tool — they tell you exactly what to look for in the student's answer. Determine the total marks for the paper by summing all QUESTION mark allocations. Note: rubric criteria marks are scoring guides that may exceed a question's allocated marks to provide flexibility — always cap the awarded marks at the question's own mark allocation."
    }));

    for (i, maybe_b64) in scheme_b64.iter().enumerate() {
        if let Some(b64) = maybe_b64 {
            scheme_parts.push(serde_json::json!({
                "inline_data": { "mime_type": "image/jpeg", "data": b64 }
            }));
        } else {
            tracing::warn!(index = i, "ai_prepare: scheme image missing");
        }
    }

    let cache_name = create_cache_with_retry(gemini, &scheme_parts).await;

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

    tracing::info!(
        paper_id = %req.paper_id,
        elapsed_ms = start.elapsed().as_millis(),
        cached = cache_name.is_some(),
        "ai_prepare: complete"
    );

    PreparedRequest {
        mark_req: req,
        cache_name,
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
                tracing::warn!(adm, attempt = attempt + 1, error = %e, "student marking attempt failed");
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap())
}

async fn mark_all_questions_with_retry(
    gemini: &GeminiClient,
    cache_name: &str,
    student_images: &[String],
    questions: &[AllQuestionInput],
) -> std::result::Result<Vec<(i32, QuestionScore)>, Box<dyn std::error::Error + Send + Sync>> {
    let delays = [0u64, 2, 4];
    let mut last_err = None;
    for (attempt, &delay_secs) in delays.iter().enumerate() {
        if delay_secs > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
        }
        match gemini
            .mark_all_questions(cache_name, student_images, questions)
            .await
        {
            Ok(scores) => return Ok(scores),
            Err(e) => {
                tracing::warn!(attempt = attempt + 1, error = %e, "all-questions marking attempt failed");
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
            mark_student_with_retry(&client, &cn, adm, &images).await
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
    cache_name: &str,
    student_images: &[(i32, Vec<String>)],
    paper_id: &str,
) -> Vec<crate::ai::gemini::StudentScore> {
    let display_name = format!("marking-{}", paper_id);

    let students_ref: Vec<(i32, &[String])> = student_images
        .iter()
        .map(|(adm, imgs)| (*adm, imgs.as_slice()))
        .collect();

    let batch_name = match gemini
        .create_batch_job(cache_name, &students_ref, &display_name)
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

    let gemini_cleanup = gemini.clone();
    let bn = batch_name.clone();
    tokio::spawn(async move {
        gemini_cleanup.delete_batch_job(&bn).await;
    });

    scores
}

// ---------------------------------------------------------------------------
// Route marking: per-question (question bank) or legacy (whole-paper)
// ---------------------------------------------------------------------------

async fn route_marking(
    gemini: &GeminiClient,
    prepared: PreparedRequest,
) -> std::result::Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let has_paper_questions = CONN.with(|conn| {
        match question_bank::get_paper_questions(conn, &prepared.mark_req.paper_id, None) {
            Ok(pqs) => !pqs.is_empty(),
            Err(_) => false,
        }
    });

    if has_paper_questions {
        tracing::info!(
            paper_id = %prepared.mark_req.paper_id,
            "ai_worker: routing to per-question marking"
        );
        mark_and_write_per_question(gemini, prepared).await
    } else {
        mark_and_write(gemini, prepared).await
    }
}

// ---------------------------------------------------------------------------
// Per-question marking flow
// ---------------------------------------------------------------------------

async fn mark_and_write_per_question(
    gemini: &GeminiClient,
    prepared: PreparedRequest,
) -> std::result::Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let task_start = Instant::now();
    let paper_id = prepared.mark_req.paper_id.clone();
    let total_students = prepared.student_images.len();

    tracing::info!(
        paper_id = %paper_id,
        student_count = total_students,
        "ai_pq: starting per-question marking"
    );

    // 1. Load paper_questions
    let paper_qs = match CONN.with(|conn| {
        question_bank::get_paper_questions(conn, &paper_id, None)
    }) {
        Ok(pqs) => pqs,
        Err(e) => return Err(format!("Failed to load paper questions: {:?}", e).into()),
    };

    if paper_qs.is_empty() {
        return Err("No paper_questions found for per-question marking".into());
    }

    tracing::info!(
        question_count = paper_qs.len(),
        "ai_pq: paper questions loaded"
    );

    // 2. Init marking queue
    CONN.with(|conn| {
        let _ = question_bank::upsert_marking_queue(conn, &paper_id);
        let _ = question_bank::update_marking_status(
            conn,
            &paper_id,
            1,
            "Downloading images...",
            None,
            total_students as i32,
            0,
        );
    });

    // 3. Load question data
    struct QuestionData {
        id: i32,
        body: String,
        marks: i16,
        rubric: Vec<(String, i16)>,
        images_b64: Vec<(String, Option<String>)>,
    }

    let raw_data = match CONN.with(|conn| -> crate::types::error::Result<Vec<_>> {
        let mut out = Vec::with_capacity(paper_qs.len());
        for pq in &paper_qs {
            let q = question_bank::get_question(conn, pq.question)?.ok_or(Error::NotFound)?;
            let rubric = question_bank::get_rubric_criteria(conn, pq.question)?;
            let images = question_bank::get_question_images(conn, pq.question)?;
            out.push((q, rubric, images));
        }
        Ok(out)
    }) {
        Ok(data) => data,
        Err(e) => return Err(format!("Failed to load question data: {:?}", e).into()),
    };

    let mut questions_data: Vec<QuestionData> = Vec::with_capacity(paper_qs.len());
    for (q, rubric, images) in raw_data {
        let rubric_pairs: Vec<(String, i16)> = rubric
            .iter()
            .map(|r| (r.criterion.clone(), r.marks))
            .collect();

        let mut imgs_b64 = Vec::new();
        for img_row in &images {
            let get_url = sign::url(&img_row.key, sign::GET_TTL, false);
            match gemini.download_b64(&get_url).await {
                Ok(b64) => imgs_b64.push((b64, img_row.caption.clone())),
                Err(e) => {
                    tracing::warn!(
                        question = q.id.unwrap_or(0),
                        key = %img_row.key,
                        error = %e,
                        "ai_pq: question image download failed — skipping"
                    );
                }
            }
        }

        questions_data.push(QuestionData {
            id: q.id.unwrap_or(0),
            body: q.body.clone(),
            marks: q.marks,
            rubric: rubric_pairs,
            images_b64: imgs_b64,
        });
    }

    // 4. Ensure context cache
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

    let cache_name = match prepared.cache_name {
        Some(cn) => cn,
        None => {
            let scheme_parts = gemini
                .build_scheme_parts(&prepared.mark_req.scheme_get_urls)
                .await?;
            match create_cache_with_retry(gemini, &scheme_parts).await {
                Some(cn) => cn,
                None => {
                    CONN.with(|conn| {
                        let _ = question_bank::update_marking_status(
                            conn,
                            &paper_id,
                            6,
                            "Cache creation failed",
                            Some("Failed to create Gemini context cache after 3 attempts"),
                            total_students as i32,
                            0,
                        );
                    });
                    return Err("Failed to create context cache for per-question marking".into());
                }
            }
        }
    };

    // 5. Mark all students
    let all_q_inputs: Vec<AllQuestionInput> = questions_data
        .iter()
        .map(|qd| AllQuestionInput {
            question_id: qd.id,
            text: qd.body.clone(),
            marks: qd.marks,
            rubric: qd.rubric.clone(),
            images_b64: qd.images_b64.clone(),
        })
        .collect();

    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT));
    let mut handles = Vec::with_capacity(prepared.student_images.len());

    for (adm, images) in &prepared.student_images {
        let client = gemini.clone();
        let sem = Arc::clone(&semaphore);
        let adm = *adm;
        let images = images.clone();
        let cn = cache_name.clone();
        let qs = all_q_inputs.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore closed");
            mark_all_questions_with_retry(&client, &cn, &images, &qs).await
        });
        handles.push((adm, handle));
    }

    let mut marked_count: i32 = 0;

    for (adm, handle) in handles {
        match handle.await {
            Ok(Ok(question_scores)) => {
                for (question_id, score) in &question_scores {
                    let _ = CONN.with(|conn| {
                        question_bank::upsert_question_grade(
                            conn,
                            &paper_id,
                            adm,
                            *question_id,
                            score.score as f32,
                            Some(&score.feedback),
                            None,
                        )
                    });
                    tracing::debug!(
                        adm,
                        question = question_id,
                        score = score.score,
                        "ai_pq: question graded"
                    );
                }
                marked_count += 1;
                let progress = format!("{}/{} students marked", marked_count, total_students);
                let _ = CONN.with(|conn| {
                    question_bank::update_marking_status(
                        conn,
                        &paper_id,
                        3,
                        &progress,
                        None,
                        total_students as i32,
                        marked_count,
                    )
                });
            }
            Ok(Err(e)) => {
                tracing::error!(adm, error = %e, "ai_pq: all-questions marking failed — student skipped");
            }
            Err(e) => {
                tracing::error!(adm, error = %e, "ai_pq: all-questions task panicked — student skipped");
            }
        }
    }

    // 6. Aggregate per-question grades into paper total
    CONN.with(|conn| {
        let _ = question_bank::update_marking_status(
            conn,
            &paper_id,
            4,
            "Aggregating scores...",
            None,
            total_students as i32,
            marked_count,
        );
    });

    let total_paper_marks: i16 = questions_data.iter().map(|qd| qd.marks).sum();
    let mut student_scores: Vec<crate::ai::gemini::StudentScore> =
        Vec::with_capacity(total_students);

    for (adm, _) in &prepared.student_images {
        let grades = match CONN.with(|conn| {
            question_bank::get_question_grades_for_student(conn, &paper_id, *adm)
        }) {
            Ok(g) => g,
            Err(e) => {
                tracing::warn!(adm, error = ?e, "ai_pq: failed to load question grades");
                continue;
            }
        };

        if grades.is_empty() {
            continue;
        }

        let total_score: f64 = grades.iter().map(|g| g.score as f64).sum();
        let breakdown: Vec<crate::ai::gemini::QuestionBreakdown> = grades
            .iter()
            .map(|g| {
                let out_of = questions_data
                    .iter()
                    .find(|q| q.id == g.question)
                    .map(|q| q.marks as f64)
                    .unwrap_or(0.0);
                crate::ai::gemini::QuestionBreakdown {
                    question: g.question.to_string(),
                    awarded: g.score as f64,
                    out_of,
                    note: g.feedback.clone().unwrap_or_default(),
                }
            })
            .collect();

        student_scores.push(crate::ai::gemini::StudentScore {
            adm: *adm,
            score: total_score,
            total: total_paper_marks as i32,
            breakdown,
        });
    }

    let grades_written = if !student_scores.is_empty() {
        write_grades_to_db(&paper_id, &student_scores)
    } else {
        0
    };

    // 7. Complete
    CONN.with(|conn| {
        let _ = question_bank::update_marking_status(
            conn,
            &paper_id,
            5,
            "Complete",
            None,
            total_students as i32,
            marked_count,
        );
    });

    let gemini_cleanup = gemini.clone();
    let cn = cache_name.clone();
    tokio::spawn(async move {
        gemini_cleanup.delete_context_cache(&cn).await;
    });

    tracing::info!(
        paper_id = %paper_id,
        students_marked = marked_count,
        total_students,
        grades_written,
        elapsed_ms = task_start.elapsed().as_millis(),
        "ai_pq: per-question marking complete"
    );

    Ok(grades_written)
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
    let cache_name = prepared.cache_name.as_deref();

    tracing::info!(
        paper_id = %paper_id,
        student_count,
        cached = cache_name.is_some(),
        "ai_mark: starting student marking"
    );

    if cache_name.is_none() {
        tracing::info!("ai_mark: no cache — falling back to non-cached mark_paper");
        let scheme_urls = &prepared.mark_req.scheme_get_urls;
        let students = &prepared.mark_req.students;
        let scores = match gemini.mark_paper(scheme_urls, students).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = %e, "ai_mark: fallback mark_paper failed");
                return Err(e);
            }
        };
        let count = write_grades_to_db(paper_id, &scores);
        return Ok(count);
    }

    let cache_name_str = cache_name.unwrap();

    let (scores, failed_adms) = if student_count > 1 {
        tracing::info!(student_count, "ai_mark: using batch API");
        let batch_scores =
            mark_batch_with_fallback(gemini, cache_name_str, &prepared.student_images, paper_id)
                .await;

        if !batch_scores.is_empty() {
            (batch_scores, Vec::new())
        } else {
            tracing::warn!("ai_mark: batch failed — falling back to real-time");
            mark_students_realtime(gemini, cache_name_str, &prepared.student_images).await
        }
    } else {
        mark_students_realtime(gemini, cache_name_str, &prepared.student_images).await
    };

    if !failed_adms.is_empty() {
        tracing::warn!(
            failed_count = failed_adms.len(),
            total_count = student_count,
            "ai_mark: some students failed"
        );
    }

    let count = if !scores.is_empty() {
        write_grades_to_db(paper_id, &scores)
    } else {
        0
    };

    let gemini_cleanup = gemini.clone();
    let cn = cache_name_str.to_string();
    tokio::spawn(async move {
        gemini_cleanup.delete_context_cache(&cn).await;
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
                // Append changelog record for grade
                let log_user = Id::system();
                LOG.with(|cell| {
                    let mut log = cell.borrow_mut();
                    // TBL_GRADES = 18 (from old constant, kept for changelog compatibility)
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
