use rand::RngExt;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use super::gemini::{QuestionBreakdown, StudentScore};

const MODEL: &str = "claude-sonnet-4-6-20250514";
const BASE_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

/// Max concurrent Anthropic API requests when marking multiple students.
const MAX_CONCURRENT: usize = 4;

#[derive(Clone)]
pub struct AnthropicClient {
    http: reqwest::Client,
    api_key: &'static str,
}

// -- Anthropic API response envelope --

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

// -- Marking result deserialization (mirrors gemini.rs private types) --

#[derive(Deserialize)]
struct MarkingResult {
    results: Vec<ScoreEntry>,
}

#[derive(Deserialize)]
struct ScoreEntry {
    adm: i32,
    score: f64,
    total: i32,
    #[serde(default)]
    breakdown: Vec<BreakdownEntry>,
}

#[derive(Deserialize)]
struct BreakdownEntry {
    q: serde_json::Value,
    awarded: f64,
    out_of: f64,
    #[serde(default)]
    note: String,
}

/// Local context cache: stores scheme parts keyed by a UUID string.
/// We store scheme_parts in-process so we can include them in every request.
/// Anthropic's server-side prompt caching kicks in automatically via
/// `cache_control` breakpoints on the system instruction and last scheme image,
/// giving us a ~5 min KV-cache hit window for repeated requests with the same prefix.
static CACHE: std::sync::LazyLock<tokio::sync::RwLock<HashMap<String, Vec<serde_json::Value>>>> =
    std::sync::LazyLock::new(|| tokio::sync::RwLock::new(HashMap::new()));

const SYSTEM_INSTRUCTION: &str = r#"You are an expert national-exam marker for Kenyan secondary school examinations (KCSE and equivalent). You mark ALL subjects: Mathematics, Sciences (Biology, Chemistry, Physics), Languages (English, Kiswahili), Humanities (History, Geography, CRE, IRE, HRE), and Technical subjects (Business Studies, Agriculture, Computer Studies, Home Science).

You will receive a marking scheme and ONE student's answer sheets. Mark ONLY this student.

## Your Marking Process

### Step 1 — Analyse the Marking Scheme
- Read every page of the marking scheme carefully.
- For each question/sub-question, identify: the mark allocation (e.g. M1, A1, B1, or just "1 mark"), the expected answer or acceptable range, and any special rubric instructions.
- Determine the TOTAL marks for the paper by summing all individual mark allocations.
- Note any follow-through (FT) annotations, alternative acceptable answers, or "Accept" notes.

### Step 2 — Mark the Student
- Go through EVERY question in the marking scheme order.
- For each mark point, decide whether to award it based on the rules below.
- Record the marks awarded per question and a brief justification.
- Sum the per-question marks to get the student's total score.

## Mark Types

- **B marks** (standalone): Awarded for a correct result, fact, or statement. No working is required.
- **M marks** (method): Awarded for a correct method, procedure, or approach — even if the numbers used are wrong (provided they come from the student's own earlier working). M marks reward the PROCESS, not the answer.
- **A marks** (accuracy): Awarded for a correct final answer following correct method. An A mark CANNOT be awarded if the preceding M mark was not earned — UNLESS the rubric explicitly indicates otherwise.
- **FT marks** (follow-through): Awarded when a student uses their own incorrect value from an earlier step and applies correct method/reasoning with it in subsequent steps.

If the marking scheme does not use M/A/B notation, infer the mark type from context: method steps get M-type treatment, final answers get A-type treatment, standalone facts get B-type treatment.

## Follow-Through Marking Rules (CRITICAL)

This is the most important marking principle. Apply it rigorously:

1. If a student makes an error in step N (losing that mark), but then uses their incorrect result correctly in steps N+1, N+2, etc., award FULL METHOD MARKS for those subsequent steps.
2. Only deduct marks at the POINT OF ERROR — not at every downstream step that uses the wrong value.
3. A follow-through accuracy mark (A-FT) is awarded when the student's final answer is correct relative to their own earlier working, even though it differs from the model answer.
4. Follow-through does NOT apply if:
   - The error makes subsequent working trivial or meaningless (e.g., getting 0 when a division was required).
   - The student changes approach entirely and does not build on their earlier work.
   - The rubric explicitly states "No FT" for that mark.

Example: If the marking scheme awards M1 for method, A1 for correct answer on a two-step problem, and the student makes an arithmetic slip in step 1 (losing A1) but applies perfectly correct method in step 2 using their wrong value — award the M1 for step 2 and the A1-FT if their final answer is consistent with their earlier error.

## Benefit of the Doubt

- If handwriting is ambiguous between two characters (e.g., 5 vs 6, x-squared vs x-cubed), choose the interpretation that is more consistent with the student's surrounding working and the expected answer.
- If still genuinely ambiguous after considering context, AWARD the mark.
- Never penalise a student for poor handwriting if the intended content can be reasonably inferred.

## Equivalent Forms

Accept ANY mathematically, scientifically, or linguistically equivalent answer unless the rubric EXPLICITLY requires a specific form. Examples:
- 4*sqrt(2) = sqrt(32) = approx 5.657 (all equivalent)
- 3x + 2y = 4 is the same as y = -3x/2 + 2 is the same as 3x + 2y - 4 = 0
- 0.5 = 1/2 = 50 percent
- "photosynthesis produces glucose" is equivalent to "plants make food using light energy"
- Correct chemical formulae in any standard notation

## Diagrams, Graphs, and Sketches

- Assess the CONTENT, not artistic quality.
- For graphs: check axes labels, scale consistency, plotted points, line/curve shape, and title.
- For diagrams: check that all required parts are labelled and relationships are shown correctly.
- For number lines: check direction, spacing proportionality, and marked values.
- Neatness is NEVER a marking criterion unless the rubric explicitly states it.

## Presentation and Layout

- Do NOT penalise for non-standard working layout, crossing out, or using margins.
- If a student crosses out work but does not replace it, mark the crossed-out work.
- If a student provides two different answers to the same question without crossing either out, mark the FIRST answer only.

## Rounding and Units

- Accept any correct rounding to the same or greater precision as the rubric answer, unless the rubric specifies exact decimal places or significant figures.
- Award marks for correct numerical answers even if units are missing — UNLESS the rubric explicitly allocates a mark for units.

## Language and Essay Subjects

For English, Kiswahili, and essay-based questions in any subject:
- Mark according to the rubric's assessment criteria (content, language, organisation, etc.).
- Award marks within the rubric's band descriptors.
- Do not penalise for dialect variations that are acceptable in the Kenyan curriculum context.

## Output Format

Return ONLY a JSON object with exactly one entry in the results array (for the single student you are marking):

{"results": [{"adm": <integer>, "score": <number>, "total": <integer>, "breakdown": [{"q": "<question number>", "awarded": <number>, "out_of": <number>, "note": "<one-sentence justification>"}]}]}

Rules:
- Every question in the marking scheme MUST appear in the breakdown.
- The sum of all "awarded" values MUST equal "score".
- The sum of all "out_of" values MUST equal "total".
- The "note" should be concise: what was awarded and why, especially noting follow-through or deductions."#;

// -- Implementation --

impl AnthropicClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: env!("ANTHROPIC_API_KEY"),
        }
    }

    /// Download an image from a URL and return it as a base64-encoded string.
    pub async fn download_b64(
        &self,
        url: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;
        let bytes = self.http.get(url).send().await?.bytes().await?;
        Ok(STANDARD.encode(&bytes))
    }

    /// Build the shared marking-scheme content parts (text + base64 images).
    /// These are identical for every student and are computed once.
    /// Returns Anthropic-format content blocks.
    pub async fn build_scheme_parts(
        &self,
        scheme_urls: &[String],
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        let mut parts = Vec::with_capacity(scheme_urls.len() + 1);

        parts.push(serde_json::json!({
            "type": "text",
            "text": "## MARKING SCHEME\n\nThe following images contain the marking scheme for this paper. Study them carefully to identify every question, sub-question, mark allocation, expected answer, and any rubric notes (such as FT, Accept, OR, etc.). Determine the total marks for the paper by summing all mark allocations."
        }));

        for (i, url) in scheme_urls.iter().enumerate() {
            eprintln!(
                "[ANTHROPIC] downloading scheme image {}/{}",
                i + 1,
                scheme_urls.len()
            );
            tracing::debug!(index = i, "anthropic: downloading scheme image");
            let b64 = self.download_b64(url).await?;
            tracing::debug!(index = i, "anthropic: scheme image downloaded");
            if i == scheme_urls.len() - 1 {
                // Last scheme image: add cache_control breakpoint so Anthropic
                // caches everything up to and including this block server-side.
                parts.push(serde_json::json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/jpeg",
                        "data": b64
                    },
                    "cache_control": {"type": "ephemeral"}
                }));
            } else {
                parts.push(serde_json::json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/jpeg",
                        "data": b64
                    }
                }));
            }
        }

        Ok(parts)
    }

    /// Mark a single student against the prebuilt scheme parts.
    /// Returns a single StudentScore.
    async fn mark_single_student(
        &self,
        scheme_parts: &[serde_json::Value],
        adm: i32,
        answer_urls: &[String],
    ) -> Result<StudentScore, Box<dyn std::error::Error + Send + Sync>> {
        // Build per-student content: scheme parts + student answer sheets + final instruction
        let mut content_blocks = Vec::with_capacity(scheme_parts.len() + answer_urls.len() + 3);

        // 1. Scheme parts (shared, cloned)
        content_blocks.extend_from_slice(scheme_parts);

        // 2. Student answer sheets
        content_blocks.push(serde_json::json!({
            "type": "text",
            "text": format!("## STUDENT ANSWER SHEETS\n\nMark the following student's answer sheets against the marking scheme above. Apply all marking rules — especially follow-through marking.\n\n### Student ADM {}:", adm)
        }));

        for (i, url) in answer_urls.iter().enumerate() {
            eprintln!(
                "[ANTHROPIC] downloading answer sheet for adm={} page={}",
                adm,
                i + 1
            );
            tracing::debug!(
                adm = adm,
                sheet = i,
                "anthropic: downloading student answer sheet"
            );
            let b64 = self.download_b64(url).await?;
            tracing::debug!(adm = adm, sheet = i, "anthropic: student sheet downloaded");
            content_blocks.push(serde_json::json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": "image/jpeg",
                    "data": b64
                }
            }));
        }

        // 3. Final instruction
        let final_instruction = format!(
            r#"## FINAL INSTRUCTIONS

Mark student ADM {} against the marking scheme above.

Remember:
- Apply follow-through (FT) marking: only deduct at the point of error, not at every subsequent step.
- Accept equivalent forms unless the rubric explicitly requires a specific form.
- Give benefit of the doubt on ambiguous handwriting.
- The total marks must be determined from the marking scheme (sum of all mark allocations).

Return ONLY valid JSON with exactly one result entry for this student:

{{"results": [{{"adm": {}, "score": <marks_awarded>, "total": <total_marks_for_paper>, "breakdown": [{{"q": "<question number>", "awarded": <number>, "out_of": <number>, "note": "<one-sentence justification>"}}]}}]}}"#,
            adm, adm
        );
        content_blocks.push(serde_json::json!({ "type": "text", "text": final_instruction }));

        // 4. Build request body — temperature 0 for deterministic marking
        let body = serde_json::json!({
            "model": MODEL,
            "max_tokens": 16384,
            "temperature": 0,
            "system": [
                {
                    "type": "text",
                    "text": SYSTEM_INSTRUCTION,
                    "cache_control": {"type": "ephemeral"}
                }
            ],
            "messages": [
                {
                    "role": "user",
                    "content": content_blocks
                }
            ]
        });

        let request_size = body.to_string().len();

        eprintln!(
            "[ANTHROPIC] ADM {}: sending POST ({} bytes)",
            adm, request_size
        );
        tracing::info!(
            model = MODEL,
            adm = adm,
            request_body_bytes = request_size,
            "anthropic: sending per-student POST"
        );

        let post_start = Instant::now();
        let response = self
            .http
            .post(BASE_URL)
            .header("x-api-key", self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;

        eprintln!(
            "[ANTHROPIC] ADM {}: response HTTP {} ({} bytes) in {}ms",
            adm,
            status.as_u16(),
            text.len(),
            post_start.elapsed().as_millis()
        );
        tracing::info!(
            model = MODEL,
            adm = adm,
            http_status = status.as_u16(),
            response_bytes = text.len(),
            elapsed_ms = post_start.elapsed().as_millis(),
            "anthropic: received per-student response"
        );

        if !status.is_success() {
            eprintln!(
                "[ANTHROPIC] ADM {}: ERROR HTTP {} — {}",
                adm,
                status.as_u16(),
                text
            );
            tracing::error!(
                model = MODEL,
                adm = adm,
                http_status = status.as_u16(),
                response_body = %text,
                "anthropic: per-student API error"
            );
            return Err(format!(
                "Anthropic API returned status {} for ADM {} — body: {}",
                status, adm, text
            )
            .into());
        }

        // Parse response
        let anthropic_resp: AnthropicResponse = serde_json::from_str(&text).map_err(|e| {
            tracing::error!(adm = adm, parse_error = %e, raw_response = %text, "anthropic: failed to parse AnthropicResponse");
            e
        })?;

        let result_text = anthropic_resp
            .content
            .iter()
            .find(|b| b.block_type == "text" && b.text.is_some())
            .and_then(|b| b.text.as_ref())
            .ok_or_else(|| {
                tracing::error!(adm = adm, raw_response = %text, "anthropic: no text in response");
                format!("No text in Anthropic response for ADM {}", adm)
            })?;

        tracing::debug!(adm = adm, result_json = %result_text, "anthropic: raw marking JSON");

        let marking: MarkingResult = serde_json::from_str(result_text).map_err(|e| {
            tracing::error!(adm = adm, parse_error = %e, raw_result = %result_text, "anthropic: failed to parse MarkingResult");
            e
        })?;

        let entry = marking.results.into_iter().next().ok_or_else(|| {
            tracing::error!(adm = adm, "anthropic: empty results array");
            format!("Anthropic returned empty results for ADM {}", adm)
        })?;

        // Log breakdown
        eprintln!(
            "[ANTHROPIC] ADM {} — score: {}/{} ({} questions)",
            entry.adm,
            entry.score,
            entry.total,
            entry.breakdown.len()
        );
        tracing::info!(
            adm = entry.adm,
            score = entry.score,
            total = entry.total,
            question_count = entry.breakdown.len(),
            "anthropic: student score summary"
        );

        for b in &entry.breakdown {
            let q_label = match &b.q {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                other => other.to_string(),
            };
            eprintln!(
                "[ANTHROPIC]   Q{}: {}/{} — {}",
                q_label, b.awarded, b.out_of, b.note
            );
            tracing::info!(
                adm = entry.adm,
                question = %q_label,
                awarded = b.awarded,
                out_of = b.out_of,
                note = %b.note,
                "anthropic: question breakdown"
            );
        }

        let breakdown = entry
            .breakdown
            .into_iter()
            .map(|b| {
                let question = match b.q {
                    serde_json::Value::String(s) => s,
                    serde_json::Value::Number(n) => n.to_string(),
                    other => other.to_string(),
                };
                QuestionBreakdown {
                    question,
                    awarded: b.awarded,
                    out_of: b.out_of,
                    note: b.note,
                }
            })
            .collect();

        Ok(StudentScore {
            adm: entry.adm,
            score: entry.score,
            total: entry.total,
            breakdown,
        })
    }

    /// Mark all students against the given marking scheme.
    ///
    /// Architecture: each student is marked in an ISOLATED Anthropic API call
    /// with only the marking scheme + that student's answer sheets. This ensures
    /// consistent scoring regardless of how many students are in the batch.
    /// Requests run concurrently (up to MAX_CONCURRENT at a time).
    pub async fn mark_paper(
        &self,
        scheme_urls: &[String],
        students: &[(i32, Vec<String>)],
    ) -> Result<Vec<StudentScore>, Box<dyn std::error::Error + Send + Sync>> {
        let total_images = scheme_urls.len() + students.iter().map(|(_, u)| u.len()).sum::<usize>();

        eprintln!(
            "[ANTHROPIC] starting: model={} scheme_images={} students={} total_images={} concurrency={}",
            MODEL,
            scheme_urls.len(),
            students.len(),
            total_images,
            MAX_CONCURRENT,
        );
        tracing::info!(
            model = MODEL,
            scheme_image_count = scheme_urls.len(),
            student_count = students.len(),
            total_image_count = total_images,
            max_concurrent = MAX_CONCURRENT,
            "anthropic: starting mark_paper (per-student isolation)"
        );

        let dl_start = Instant::now();

        // 1. Download and encode scheme images ONCE
        let scheme_parts = self.build_scheme_parts(scheme_urls).await?;
        let scheme_parts = Arc::new(scheme_parts);

        eprintln!(
            "[ANTHROPIC] scheme images ready in {}ms — launching {} per-student requests",
            dl_start.elapsed().as_millis(),
            students.len()
        );
        tracing::info!(
            scheme_elapsed_ms = dl_start.elapsed().as_millis(),
            student_count = students.len(),
            "anthropic: scheme ready — launching per-student requests"
        );

        // 2. Mark each student in a separate API call, bounded concurrency
        let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT));
        let mut handles = Vec::with_capacity(students.len());

        for (adm, urls) in students {
            let client = self.clone();
            let scheme = Arc::clone(&scheme_parts);
            let sem = Arc::clone(&semaphore);
            let adm = *adm;
            let urls = urls.clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed");
                client.mark_single_student(&scheme, adm, &urls).await
            });
            handles.push((adm, handle));
        }

        // 3. Collect results
        let mut scores = Vec::with_capacity(students.len());
        let mut errors = Vec::new();

        for (adm, handle) in handles {
            match handle.await {
                Ok(Ok(score)) => scores.push(score),
                Ok(Err(e)) => {
                    eprintln!("[ANTHROPIC] ADM {} FAILED: {}", adm, e);
                    tracing::error!(adm = adm, error = %e, "anthropic: per-student marking failed");
                    errors.push((adm, e));
                }
                Err(e) => {
                    eprintln!("[ANTHROPIC] ADM {} task panicked: {}", adm, e);
                    tracing::error!(adm = adm, error = %e, "anthropic: per-student task panicked");
                    errors.push((adm, Box::new(e) as Box<dyn std::error::Error + Send + Sync>));
                }
            }
        }

        if scores.is_empty() && !errors.is_empty() {
            let (adm, err) = errors.into_iter().next().unwrap();
            return Err(format!("All students failed. First error (ADM {}): {}", adm, err).into());
        }

        if !errors.is_empty() {
            let failed_adms: Vec<i32> = errors.iter().map(|(a, _)| *a).collect();
            eprintln!(
                "[ANTHROPIC] WARNING: {} of {} students failed: {:?}",
                errors.len(),
                students.len(),
                failed_adms
            );
            tracing::warn!(
                failed_count = errors.len(),
                total_count = students.len(),
                failed_adms = ?failed_adms,
                "anthropic: some students failed, returning partial results"
            );
        }

        eprintln!(
            "[ANTHROPIC] complete: {} scores in {}ms total",
            scores.len(),
            dl_start.elapsed().as_millis()
        );
        tracing::info!(
            model = MODEL,
            scored_count = scores.len(),
            total_elapsed_ms = dl_start.elapsed().as_millis(),
            "anthropic: mark_paper complete"
        );

        Ok(scores)
    }

    /// Create a local context cache containing scheme parts.
    /// The scheme_parts are stored in an in-process HashMap so they can be
    /// included in every per-student request. Anthropic's server-side prompt
    /// caching is triggered automatically by `cache_control` breakpoints that
    /// `build_scheme_parts` places on the last scheme image block — identical
    /// prefixes across requests hit the server KV cache (~5 min TTL).
    /// Returns a cache ID string.
    pub async fn create_context_cache(
        &self,
        scheme_parts: &[serde_json::Value],
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let random_bytes: [u8; 16] = rand::rng().random();
        let cache_id = format!("anthropic-cache/{}", hex::encode(random_bytes));

        eprintln!(
            "[ANTHROPIC] creating local context cache: {} ({} parts)",
            cache_id,
            scheme_parts.len()
        );
        tracing::info!(
            cache_name = %cache_id,
            part_count = scheme_parts.len(),
            "anthropic: creating local context cache"
        );

        CACHE
            .write()
            .await
            .insert(cache_id.clone(), scheme_parts.to_vec());

        eprintln!("[ANTHROPIC] local context cache created: {}", cache_id);
        tracing::info!(cache_name = %cache_id, "anthropic: local context cache created");

        Ok(cache_id)
    }

    /// Delete a local context cache (remove from the in-process HashMap).
    /// Anthropic's server-side prompt cache expires automatically (~5 min TTL),
    /// so this only cleans up the local in-process storage.
    pub async fn delete_context_cache(&self, cache_name: &str) {
        eprintln!("[ANTHROPIC] deleting local context cache: {}", cache_name);
        tracing::info!(cache_name = %cache_name, "anthropic: deleting local context cache");

        let removed = CACHE.write().await.remove(cache_name).is_some();

        if removed {
            eprintln!("[ANTHROPIC] cache deleted: {}", cache_name);
            tracing::info!(cache_name = %cache_name, "anthropic: cache deleted");
        } else {
            eprintln!(
                "[ANTHROPIC] cache not found (already deleted?): {}",
                cache_name
            );
            tracing::warn!(cache_name = %cache_name, "anthropic: cache not found for deletion");
        }
    }

    /// Mark a single student using a pre-created local context cache.
    /// The cache contains the scheme parts; only the student's answer sheets
    /// and final instruction need to be appended.
    pub async fn mark_student_cached(
        &self,
        cache_name: &str,
        adm: i32,
        answer_images_b64: &[String],
    ) -> Result<StudentScore, Box<dyn std::error::Error + Send + Sync>> {
        // Look up scheme parts from local cache
        let scheme_parts = {
            let guard = CACHE.read().await;
            guard.get(cache_name).cloned().ok_or_else(|| {
                let msg = format!("Anthropic local cache not found: {}", cache_name);
                tracing::error!(cache_name = %cache_name, "anthropic: cache not found");
                msg
            })?
        };

        let mut content_blocks =
            Vec::with_capacity(scheme_parts.len() + answer_images_b64.len() + 3);

        // 1. Scheme parts from cache
        content_blocks.extend_from_slice(&scheme_parts);

        // 2. Student answer sheets (already base64-encoded)
        content_blocks.push(serde_json::json!({
            "type": "text",
            "text": format!("## STUDENT ANSWER SHEETS\n\nMark the following student's answer sheets against the marking scheme above. Apply all marking rules — especially follow-through marking.\n\n### Student ADM {}:", adm)
        }));

        for b64 in answer_images_b64 {
            content_blocks.push(serde_json::json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": "image/jpeg",
                    "data": b64
                }
            }));
        }

        // 3. Final instruction
        let final_instruction = format!(
            r#"## FINAL INSTRUCTIONS

Mark student ADM {} against the marking scheme above.

Remember:
- Apply follow-through (FT) marking: only deduct at the point of error, not at every subsequent step.
- Accept equivalent forms unless the rubric explicitly requires a specific form.
- Give benefit of the doubt on ambiguous handwriting.
- The total marks must be determined from the marking scheme (sum of all mark allocations).

Return ONLY valid JSON with exactly one result entry for this student:

{{"results": [{{"adm": {}, "score": <marks_awarded>, "total": <total_marks_for_paper>, "breakdown": [{{"q": "<question number>", "awarded": <number>, "out_of": <number>, "note": "<one-sentence justification>"}}]}}]}}"#,
            adm, adm
        );
        content_blocks.push(serde_json::json!({ "type": "text", "text": final_instruction }));

        // 4. Build request body — temperature 0 for deterministic marking
        //    System instruction has cache_control so it's cached server-side.
        //    Scheme parts (from local cache) already have cache_control on the
        //    last image block, so the entire prefix is cached by Anthropic.
        let body = serde_json::json!({
            "model": MODEL,
            "max_tokens": 16384,
            "temperature": 0,
            "system": [
                {
                    "type": "text",
                    "text": SYSTEM_INSTRUCTION,
                    "cache_control": {"type": "ephemeral"}
                }
            ],
            "messages": [
                {
                    "role": "user",
                    "content": content_blocks
                }
            ]
        });

        let request_size = body.to_string().len();

        eprintln!(
            "[ANTHROPIC] ADM {} (cached): sending POST ({} bytes)",
            adm, request_size
        );
        tracing::info!(
            model = MODEL,
            adm = adm,
            cache_name = %cache_name,
            request_body_bytes = request_size,
            "anthropic: sending cached per-student POST"
        );

        let post_start = Instant::now();
        let response = self
            .http
            .post(BASE_URL)
            .header("x-api-key", self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;

        eprintln!(
            "[ANTHROPIC] ADM {} (cached): response HTTP {} ({} bytes) in {}ms",
            adm,
            status.as_u16(),
            text.len(),
            post_start.elapsed().as_millis()
        );
        tracing::info!(
            model = MODEL,
            adm = adm,
            http_status = status.as_u16(),
            response_bytes = text.len(),
            elapsed_ms = post_start.elapsed().as_millis(),
            "anthropic: received cached per-student response"
        );

        if !status.is_success() {
            eprintln!(
                "[ANTHROPIC] ADM {} (cached): ERROR HTTP {} — {}",
                adm,
                status.as_u16(),
                text
            );
            tracing::error!(
                model = MODEL,
                adm = adm,
                http_status = status.as_u16(),
                response_body = %text,
                "anthropic: cached per-student API error"
            );
            return Err(format!(
                "Anthropic API returned status {} for ADM {} (cached) — body: {}",
                status, adm, text
            )
            .into());
        }

        // Parse response — same logic as mark_single_student
        let anthropic_resp: AnthropicResponse = serde_json::from_str(&text).map_err(|e| {
            tracing::error!(adm = adm, parse_error = %e, raw_response = %text, "anthropic: failed to parse AnthropicResponse (cached)");
            e
        })?;

        let result_text = anthropic_resp
            .content
            .iter()
            .find(|b| b.block_type == "text" && b.text.is_some())
            .and_then(|b| b.text.as_ref())
            .ok_or_else(|| {
                tracing::error!(adm = adm, raw_response = %text, "anthropic: no text in cached response");
                format!("No text in Anthropic cached response for ADM {}", adm)
            })?;

        tracing::debug!(adm = adm, result_json = %result_text, "anthropic: raw cached marking JSON");

        let marking: MarkingResult = serde_json::from_str(result_text).map_err(|e| {
            tracing::error!(adm = adm, parse_error = %e, raw_result = %result_text, "anthropic: failed to parse cached MarkingResult");
            e
        })?;

        let entry = marking.results.into_iter().next().ok_or_else(|| {
            tracing::error!(adm = adm, "anthropic: empty results array (cached)");
            format!("Anthropic returned empty results for ADM {} (cached)", adm)
        })?;

        eprintln!(
            "[ANTHROPIC] ADM {} (cached) — score: {}/{} ({} questions)",
            entry.adm,
            entry.score,
            entry.total,
            entry.breakdown.len()
        );
        tracing::info!(
            adm = entry.adm,
            score = entry.score,
            total = entry.total,
            question_count = entry.breakdown.len(),
            "anthropic: cached student score summary"
        );

        for b in &entry.breakdown {
            let q_label = match &b.q {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                other => other.to_string(),
            };
            tracing::info!(
                adm = entry.adm,
                question = %q_label,
                awarded = b.awarded,
                out_of = b.out_of,
                note = %b.note,
                "anthropic: cached question breakdown"
            );
        }

        let breakdown = entry
            .breakdown
            .into_iter()
            .map(|b| {
                let question = match b.q {
                    serde_json::Value::String(s) => s,
                    serde_json::Value::Number(n) => n.to_string(),
                    other => other.to_string(),
                };
                QuestionBreakdown {
                    question,
                    awarded: b.awarded,
                    out_of: b.out_of,
                    note: b.note,
                }
            })
            .collect();

        Ok(StudentScore {
            adm: entry.adm,
            score: entry.score,
            total: entry.total,
            breakdown,
        })
    }
}
