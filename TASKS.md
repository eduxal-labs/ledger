# TASKS.md

## Refactor: All-at-Once Per-Question Marking

### Background

The current per-question marking flow calls `mark_single_question` once per question per student, resulting in `students × questions` Gemini API calls. For a class of 30 students with 10 questions that is 300 API calls — expensive and slow.

The goal is to change this so that each student's entire paper is marked in **one single Gemini API call** that receives all questions and rubric criteria at once and returns all question scores in one JSON response. This reduces API calls from `students × questions` to just `students`.

---

## Track A — Gemini Client Layer

### Task 01: Add `AllQuestionInput` struct and `mark_all_questions` to `GeminiClient`

**Files to create/modify:** `src/ai/gemini.rs`
**Reference files to read:** `src/ai/gemini.rs` — read `mark_single_question` (L1052–1217) and `mark_student_cached` (L881–1042) for patterns to follow
**Depends on:** None
**Parallel group:** P1

**Specification:**

#### 1. Add public input struct

Add this **after** the existing `QuestionScore` struct (around L95):

```rust
/// Input descriptor for one question in the all-at-once marking call.
#[derive(Clone)]
pub struct AllQuestionInput {
    pub question_id: i32,
    pub text: String,
    pub marks: i16,
    /// (criterion text, marks for this criterion)
    pub rubric: Vec<(String, i16)>,
    /// (base64-encoded image data, optional caption)
    pub images_b64: Vec<(String, Option<String>)>,
}
```

#### 2. Add private deserialization structs

Add these **after** the existing `SingleQuestionResult` struct (around L102):

```rust
/// Top-level JSON wrapper for the all-questions API response.
#[derive(Deserialize)]
struct AllQuestionsApiResult {
    results: Vec<AllQuestionsEntry>,
}

/// Per-question entry in the all-questions API response.
#[derive(Deserialize)]
struct AllQuestionsEntry {
    question_id: i32,
    score: f64,
    feedback: String,
}
```

#### 3. Add `mark_all_questions` method

Add this **after** the existing `mark_single_question` method (after L1217) inside `impl GeminiClient`:

```rust
/// Mark ALL questions for a single student in one API call.
///
/// Sends one `generateContent` request that includes:
/// - The student's answer sheet images
/// - Every question with its rubric criteria as structured text
/// - Any per-question images embedded inline
///
/// Returns `Vec<(question_id, QuestionScore)>` with one entry per question.
pub async fn mark_all_questions(
    &self,
    cache_name: &str,
    student_images_b64: &[String],
    questions: &[AllQuestionInput],
) -> std::result::Result<Vec<(i32, QuestionScore)>, Box<dyn std::error::Error + Send + Sync>> {
```

**Implementation steps inside the method:**

**Step A — Build `parts` starting with student answer sheet images:**
```rust
let mut parts: Vec<serde_json::Value> = Vec::new();

for img in student_images_b64 {
    parts.push(serde_json::json!({
        "inline_data": { "mime_type": "image/jpeg", "data": img }
    }));
}
```

**Step B — For each question, push a structured text block followed by any question images:**

For each `q` in `questions`:
1. Build the rubric string:
   ```
   1. {criterion} ({marks} marks)
   2. {criterion} ({marks} marks)
   ...
   ```
2. Push a text part with this layout (use `format!`):
   ```
   QUESTION {i+1} (ID: {question_id}, Total: {marks} marks):
   {text}

   Rubric:
   {rubric_string}
   ---
   ```
3. For each `(img_b64, caption)` in `q.images_b64`:
   - Push an `inline_data` image part.
   - If `caption` is `Some`, push a text part: `"Image caption: {caption}"`.

**Step C — Push the final instruction text part:**
```
Mark every question listed above for this student. Find each question's answer in the student's answer sheets shown, then score it against its rubric criteria.

Return ONLY valid JSON:
{"results": [
  {"question_id": <integer>, "score": <number>, "feedback": "<one-sentence justification>"},
  ...
]}

Rules:
- Every question_id listed above MUST appear exactly once in results.
- score for each question MUST be >= 0 and MUST NOT exceed that question's total marks.
- Partial credit is allowed and expected for partially correct answers.
```

**Step D — Build the request body** (same `cachedContent` pattern as `mark_student_cached`):
```rust
let body = serde_json::json!({
    "cachedContent": cache_name,
    "contents": [{"role": "user", "parts": parts}],
    "generationConfig": {
        "responseMimeType": "application/json",
        "temperature": 0,
        "thinkingConfig": { "thinkingLevel": "low" }
    }
});
```

**Step E — Send, log, and parse:**
- POST to `self.generate_content_url(MODEL)`.
- Log request size, HTTP status, response size, elapsed ms — use the same `eprintln!` and `tracing::info!` pattern as `mark_single_question`.
- On non-2xx status, return `Err(format!("Gemini API error {}: {}", status, text).into())`.
- Parse `GeminiResponse` → extract first non-thought text part (same pattern as every other method in this file).
- Parse `AllQuestionsApiResult` from the extracted text.
- Map to `Vec<(i32, QuestionScore)>`:
  ```rust
  Ok(result.results.into_iter().map(|e| (e.question_id, QuestionScore { score: e.score, feedback: e.feedback })).collect())
  ```
- Log one `tracing::info!` per question: `question_id`, `score`, `marks` (`q.marks`).
- Log a summary `tracing::info!` with `question_count`, `total_elapsed_ms`.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

## Track B — Service Layer

### Task 02: Refactor `mark_and_write_per_question` to use all-at-once marking

**Files to create/modify:** `src/services/ai_marking.rs`
**Reference files to read:** None — full API contract is inlined below
**Depends on:** Task 01 (parallel-safe: complete API contract inlined in this spec)
**Parallel group:** P1

**Specification:**

The following types and method will exist in `src/ai/gemini.rs` after Task 01 completes. Treat them as already available when writing this task:

```rust
// In src/ai/gemini.rs — pub types added by Task 01

#[derive(Clone)]
pub struct AllQuestionInput {
    pub question_id: i32,
    pub text: String,
    pub marks: i16,
    pub rubric: Vec<(String, i16)>,
    pub images_b64: Vec<(String, Option<String>)>,
}

impl GeminiClient {
    /// Marks all questions for one student in a single Gemini API call.
    /// Returns Vec<(question_id, QuestionScore)> — one entry per question.
    pub async fn mark_all_questions(
        &self,
        cache_name: &str,
        student_images_b64: &[String],
        questions: &[AllQuestionInput],
    ) -> std::result::Result<Vec<(i32, QuestionScore)>, Box<dyn std::error::Error + Send + Sync>>;
}
```

The existing import line at the top of `ai_marking.rs` is:
```rust
use crate::ai::gemini::{GeminiClient, QuestionScore};
```
Update it to also import `AllQuestionInput`:
```rust
use crate::ai::gemini::{AllQuestionInput, GeminiClient, QuestionScore};
```

#### Change 1: Add `mark_all_questions_with_retry`

Add this function **in place of** `mark_single_question_with_retry` (the function at around L521–564). Delete `mark_single_question_with_retry` entirely and replace it with:

```rust
/// Retry an all-questions marking call up to 3 times with exponential backoff.
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
            tracing::warn!(
                attempt = attempt + 1,
                delay_secs = delay_secs,
                "retrying all-questions marking"
            );
            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
        }
        match gemini.mark_all_questions(cache_name, student_images, questions).await {
            Ok(scores) => return Ok(scores),
            Err(e) => {
                tracing::warn!(
                    attempt = attempt + 1,
                    error = %e,
                    "all-questions marking attempt failed"
                );
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap())
}
```

#### Change 2: Rewrite step 5 (MARKING phase) inside `mark_and_write_per_question`

Locate the **MARKING phase** block inside `mark_and_write_per_question`. It currently looks like this (the section after the `// 5. MARKING phase` comment up to the `// 6. AGGREGATING phase` comment):

```rust
// Current code to DELETE:
let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT));
let mut marked_count: i32 = 0;

for (adm, images) in &prepared.student_images {
    for qd in &questions_data {
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| format!("semaphore error: {}", e))?;

        match mark_single_question_with_retry(
            gemini,
            &cache_name,
            images,
            &qd.text,
            qd.marks,
            &qd.rubric,
            &qd.images_b64,
        )
        .await
        { ... }
        drop(permit);
    }
    marked_count += 1;
    ...
}
```

Replace the entire MARKING phase block with:

```rust
// 5. MARKING phase — one API call per student, all questions at once

// Build AllQuestionInput slice once, shared across all students
let all_q_inputs: Vec<AllQuestionInput> = questions_data
    .iter()
    .map(|qd| AllQuestionInput {
        question_id: qd.id,
        text: qd.text.clone(),
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
                let _ = CONN.with(|cell| {
                    let mut conn = cell.borrow_mut();
                    question_bank::upsert_question_grade(
                        &mut conn,
                        school,
                        exam,
                        adm,
                        *question_id,
                        score.score as f32,
                        Some(&score.feedback),
                    )
                });
                tracing::debug!(
                    adm = adm,
                    question = question_id,
                    score = score.score,
                    "ai_pq: question graded"
                );
            }
            marked_count += 1;
            let progress = format!("{}/{} students marked", marked_count, total_students);
            eprintln!("[AI-PQ] {}", progress);
            let _ = CONN.with(|cell| {
                let mut conn = cell.borrow_mut();
                question_bank::update_marking_status(
                    &mut conn,
                    queue_id,
                    3,
                    &progress,
                    marked_count,
                    None,
                )
            });
        }
        Ok(Err(e)) => {
            eprintln!("[AI-PQ] ADM {} failed after retries: {}", adm, e);
            tracing::error!(
                adm = adm,
                error = %e,
                "ai_pq: all-questions marking failed after retries — student skipped"
            );
            // Do not increment marked_count — this student was not graded
        }
        Err(e) => {
            eprintln!("[AI-PQ] ADM {} task panicked: {}", adm, e);
            tracing::error!(
                adm = adm,
                error = %e,
                "ai_pq: all-questions task panicked — student skipped"
            );
        }
    }
}
```

Everything before step 5 (loading paper questions, upserting the marking queue, loading question data + images, creating the cache) and everything after step 5 (step 6 AGGREGATING phase, step 7 COMPLETE, cache cleanup) remains **completely unchanged**.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

## Completion Checklist

After both tasks are marked `[x]`:

- [ ] Run `cargo build` and confirm zero errors and zero warnings related to the changed files.
- [ ] Confirm `mark_single_question_with_retry` no longer exists in `ai_marking.rs`.
- [ ] Confirm `mark_all_questions` exists in `gemini.rs` and is `pub`.
- [ ] Confirm `AllQuestionInput` is `pub` and `Clone`.
- [ ] Orchestrator: final git commit with message `refactor(ai): mark all questions per student in one API call`.