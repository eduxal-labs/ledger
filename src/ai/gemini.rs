use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

const MODEL: &str = "gemini-3.1-flash-lite-preview";
const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const BASE_URL_CACHE: &str = "https://generativelanguage.googleapis.com/v1beta/cachedContents";

/// Max concurrent Gemini API requests when marking multiple students.
const MAX_CONCURRENT: usize = 4;

#[derive(Clone)]
pub struct GeminiClient {
    http: reqwest::Client,
    api_key: &'static str,
}

#[derive(Debug)]
pub struct StudentScore {
    pub adm: i32,
    pub score: f64,
    pub total: i32,
    pub breakdown: Vec<QuestionBreakdown>,
}

#[derive(Debug)]
pub struct QuestionBreakdown {
    pub question: String,
    pub awarded: f64,
    pub out_of: f64,
    pub note: String,
}

// -- Gemini API response envelope --

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Content,
}

#[derive(Deserialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Deserialize)]
struct Part {
    text: Option<String>,
    #[serde(default)]
    thought: bool,
}

// -- Marking result deserialization --

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

#[derive(Deserialize)]
struct CacheCreateResponse {
    name: String,
}

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

impl GeminiClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: env!("GEMINI_API_KEY"),
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
    pub async fn build_scheme_parts(
        &self,
        scheme_urls: &[String],
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        let mut parts = Vec::with_capacity(scheme_urls.len() + 1);

        parts.push(serde_json::json!({
            "text": "## MARKING SCHEME\n\nThe following images contain the marking scheme for this paper. Study them carefully to identify every question, sub-question, mark allocation, expected answer, and any rubric notes (such as FT, Accept, OR, etc.). Determine the total marks for the paper by summing all mark allocations."
        }));

        for (i, url) in scheme_urls.iter().enumerate() {
            eprintln!(
                "[GEMINI] downloading scheme image {}/{}",
                i + 1,
                scheme_urls.len()
            );
            tracing::debug!(index = i, "gemini: downloading scheme image");
            let b64 = self.download_b64(url).await?;
            tracing::debug!(index = i, "gemini: scheme image downloaded");
            parts.push(serde_json::json!({
                "inline_data": { "mime_type": "image/jpeg", "data": b64 }
            }));
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
        let mut parts = Vec::with_capacity(scheme_parts.len() + answer_urls.len() + 3);

        // 1. Scheme parts (shared, cloned)
        parts.extend_from_slice(scheme_parts);

        // 2. Student answer sheets
        parts.push(serde_json::json!({
            "text": format!("## STUDENT ANSWER SHEETS\n\nMark the following student's answer sheets against the marking scheme above. Apply all marking rules — especially follow-through marking.\n\n### Student ADM {}:", adm)
        }));

        for (i, url) in answer_urls.iter().enumerate() {
            eprintln!(
                "[GEMINI] downloading answer sheet for adm={} page={}",
                adm,
                i + 1
            );
            tracing::debug!(
                adm = adm,
                sheet = i,
                "gemini: downloading student answer sheet"
            );
            let b64 = self.download_b64(url).await?;
            tracing::debug!(adm = adm, sheet = i, "gemini: student sheet downloaded");
            parts.push(serde_json::json!({
                "inline_data": { "mime_type": "image/jpeg", "data": b64 }
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
        parts.push(serde_json::json!({ "text": final_instruction }));

        // 4. Build request body — temperature 0 for deterministic output
        let body = serde_json::json!({
            "system_instruction": {
                "parts": [{"text": SYSTEM_INSTRUCTION}]
            },
            "contents": [{"parts": parts}],
            "generationConfig": {
                "responseMimeType": "application/json",
                "temperature": 0,
                "thinkingConfig": {
                    "thinkingLevel": "high"
                }
            }
        });

        let request_size = body.to_string().len();
        let url = format!(
            "{}/{}:generateContent?key={}",
            BASE_URL, MODEL, self.api_key
        );

        eprintln!(
            "[GEMINI] ADM {}: sending POST ({} bytes)",
            adm, request_size
        );
        tracing::info!(
            model = MODEL,
            adm = adm,
            request_body_bytes = request_size,
            "gemini: sending per-student POST"
        );

        let post_start = Instant::now();
        let response = self.http.post(&url).json(&body).send().await?;
        let status = response.status();
        let text = response.text().await?;

        eprintln!(
            "[GEMINI] ADM {}: response HTTP {} ({} bytes) in {}ms",
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
            "gemini: received per-student response"
        );

        if !status.is_success() {
            eprintln!(
                "[GEMINI] ADM {}: ERROR HTTP {} — {}",
                adm,
                status.as_u16(),
                text
            );
            tracing::error!(
                model = MODEL,
                adm = adm,
                http_status = status.as_u16(),
                response_body = %text,
                "gemini: per-student API error"
            );
            return Err(format!(
                "Gemini API returned status {} for ADM {} — body: {}",
                status, adm, text
            )
            .into());
        }

        // Parse response
        let gemini_resp: GeminiResponse = serde_json::from_str(&text).map_err(|e| {
            tracing::error!(adm = adm, parse_error = %e, raw_response = %text, "gemini: failed to parse GeminiResponse");
            e
        })?;

        let result_text = gemini_resp
            .candidates
            .first()
            .and_then(|c| {
                c.content
                    .parts
                    .iter()
                    .find(|p| !p.thought && p.text.is_some())
            })
            .and_then(|p| p.text.as_ref())
            .ok_or_else(|| {
                tracing::error!(adm = adm, raw_response = %text, "gemini: no text in response");
                format!("No text in Gemini response for ADM {}", adm)
            })?;

        tracing::debug!(adm = adm, result_json = %result_text, "gemini: raw marking JSON");

        let marking: MarkingResult = serde_json::from_str(result_text).map_err(|e| {
            tracing::error!(adm = adm, parse_error = %e, raw_result = %result_text, "gemini: failed to parse MarkingResult");
            e
        })?;

        let entry = marking.results.into_iter().next().ok_or_else(|| {
            tracing::error!(adm = adm, "gemini: empty results array");
            format!("Gemini returned empty results for ADM {}", adm)
        })?;

        // Log breakdown
        eprintln!(
            "[GEMINI] ADM {} — score: {}/{} ({} questions)",
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
            "gemini: student score summary"
        );

        for b in &entry.breakdown {
            let q_label = match &b.q {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                other => other.to_string(),
            };
            eprintln!(
                "[GEMINI]   Q{}: {}/{} — {}",
                q_label, b.awarded, b.out_of, b.note
            );
            tracing::info!(
                adm = entry.adm,
                question = %q_label,
                awarded = b.awarded,
                out_of = b.out_of,
                note = %b.note,
                "gemini: question breakdown"
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
    /// Architecture: each student is marked in an ISOLATED Gemini API call
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
            "[GEMINI] starting: model={} scheme_images={} students={} total_images={} concurrency={}",
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
            "gemini: starting mark_paper (per-student isolation)"
        );

        let dl_start = Instant::now();

        // 1. Download and encode scheme images ONCE
        let scheme_parts = self.build_scheme_parts(scheme_urls).await?;
        let scheme_parts = Arc::new(scheme_parts);

        eprintln!(
            "[GEMINI] scheme images ready in {}ms — launching {} per-student requests",
            dl_start.elapsed().as_millis(),
            students.len()
        );
        tracing::info!(
            scheme_elapsed_ms = dl_start.elapsed().as_millis(),
            student_count = students.len(),
            "gemini: scheme ready — launching per-student requests"
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
                    eprintln!("[GEMINI] ADM {} FAILED: {}", adm, e);
                    tracing::error!(adm = adm, error = %e, "gemini: per-student marking failed");
                    errors.push((adm, e));
                }
                Err(e) => {
                    eprintln!("[GEMINI] ADM {} task panicked: {}", adm, e);
                    tracing::error!(adm = adm, error = %e, "gemini: per-student task panicked");
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
                "[GEMINI] WARNING: {} of {} students failed: {:?}",
                errors.len(),
                students.len(),
                failed_adms
            );
            tracing::warn!(
                failed_count = errors.len(),
                total_count = students.len(),
                failed_adms = ?failed_adms,
                "gemini: some students failed, returning partial results"
            );
        }

        eprintln!(
            "[GEMINI] complete: {} scores in {}ms total",
            scores.len(),
            dl_start.elapsed().as_millis()
        );
        tracing::info!(
            model = MODEL,
            scored_count = scores.len(),
            total_elapsed_ms = dl_start.elapsed().as_millis(),
            "gemini: mark_paper complete"
        );

        Ok(scores)
    }
    /// Create a Gemini context cache containing the system instruction and scheme images.
    /// Returns the cache name (e.g., "cachedContents/abc123").
    pub async fn create_context_cache(
        &self,
        scheme_parts: &[serde_json::Value],
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}?key={}", BASE_URL_CACHE, self.api_key);

        let body = serde_json::json!({
            "model": format!("models/{}", MODEL),
            "systemInstruction": {
                "parts": [{"text": SYSTEM_INSTRUCTION}]
            },
            "contents": [{"parts": scheme_parts, "role": "user"}],
            "ttl": "600s"
        });

        let request_size = body.to_string().len();
        eprintln!("[GEMINI] creating context cache ({} bytes)", request_size);
        tracing::info!(
            request_body_bytes = request_size,
            "gemini: creating context cache"
        );

        let start = Instant::now();
        let response = self.http.post(&url).json(&body).send().await?;
        let status = response.status();
        let text = response.text().await?;

        eprintln!(
            "[GEMINI] cache create response: HTTP {} ({} bytes) in {}ms",
            status.as_u16(),
            text.len(),
            start.elapsed().as_millis()
        );
        tracing::info!(
            http_status = status.as_u16(),
            response_bytes = text.len(),
            elapsed_ms = start.elapsed().as_millis(),
            "gemini: cache create response"
        );

        if !status.is_success() {
            eprintln!(
                "[GEMINI] cache create FAILED: HTTP {} — {}",
                status.as_u16(),
                text
            );
            tracing::error!(http_status = status.as_u16(), response_body = %text, "gemini: cache create failed");
            return Err(format!(
                "Gemini cache create returned status {} — body: {}",
                status, text
            )
            .into());
        }

        let resp: CacheCreateResponse = serde_json::from_str(&text).map_err(|e| {
            tracing::error!(parse_error = %e, raw_response = %text, "gemini: failed to parse CacheCreateResponse");
            e
        })?;

        eprintln!("[GEMINI] context cache created: {}", resp.name);
        tracing::info!(cache_name = %resp.name, "gemini: context cache created");

        Ok(resp.name)
    }

    /// Delete a Gemini context cache (best-effort, fire-and-forget).
    pub async fn delete_context_cache(&self, cache_name: &str) {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/{}?key={}",
            cache_name, self.api_key
        );

        eprintln!("[GEMINI] deleting context cache: {}", cache_name);
        tracing::info!(cache_name = %cache_name, "gemini: deleting context cache");

        match self.http.delete(&url).send().await {
            Ok(resp) => {
                eprintln!("[GEMINI] cache delete: HTTP {}", resp.status().as_u16());
                tracing::info!(
                    cache_name = %cache_name,
                    http_status = resp.status().as_u16(),
                    "gemini: cache deleted"
                );
            }
            Err(e) => {
                eprintln!("[GEMINI] cache delete failed: {}", e);
                tracing::warn!(cache_name = %cache_name, error = %e, "gemini: cache delete failed (ignored)");
            }
        }
    }

    /// Mark a single student using a pre-created context cache.
    /// The cache already contains the system instruction and scheme images,
    /// so only the student's answer sheets and final instruction are sent.
    pub async fn mark_student_cached(
        &self,
        cache_name: &str,
        adm: i32,
        answer_images_b64: &[String],
    ) -> Result<StudentScore, Box<dyn std::error::Error + Send + Sync>> {
        let mut parts = Vec::with_capacity(answer_images_b64.len() + 2);

        // Student answer sheets (already base64-encoded)
        parts.push(serde_json::json!({
            "text": format!("## STUDENT ANSWER SHEETS\n\nMark the following student's answer sheets against the marking scheme above. Apply all marking rules — especially follow-through marking.\n\n### Student ADM {}:", adm)
        }));

        for b64 in answer_images_b64 {
            parts.push(serde_json::json!({
                "inline_data": { "mime_type": "image/jpeg", "data": b64 }
            }));
        }

        // Final instruction
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
        parts.push(serde_json::json!({ "text": final_instruction }));

        // Build request using cachedContent — no system_instruction, no scheme parts
        let body = serde_json::json!({
            "cachedContent": cache_name,
            "contents": [{"parts": parts, "role": "user"}],
            "generationConfig": {
                "responseMimeType": "application/json",
                "temperature": 0,
                "thinkingConfig": {
                    "thinkingLevel": "high"
                }
            }
        });

        let request_size = body.to_string().len();
        let url = format!(
            "{}/{}:generateContent?key={}",
            BASE_URL, MODEL, self.api_key
        );

        eprintln!(
            "[GEMINI] ADM {} (cached): sending POST ({} bytes)",
            adm, request_size
        );
        tracing::info!(
            model = MODEL,
            adm = adm,
            cache_name = %cache_name,
            request_body_bytes = request_size,
            "gemini: sending cached per-student POST"
        );

        let post_start = Instant::now();
        let response = self.http.post(&url).json(&body).send().await?;
        let status = response.status();
        let text = response.text().await?;

        eprintln!(
            "[GEMINI] ADM {} (cached): response HTTP {} ({} bytes) in {}ms",
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
            "gemini: received cached per-student response"
        );

        if !status.is_success() {
            eprintln!(
                "[GEMINI] ADM {} (cached): ERROR HTTP {} — {}",
                adm,
                status.as_u16(),
                text
            );
            tracing::error!(
                model = MODEL,
                adm = adm,
                http_status = status.as_u16(),
                response_body = %text,
                "gemini: cached per-student API error"
            );
            return Err(format!(
                "Gemini API returned status {} for ADM {} (cached) — body: {}",
                status, adm, text
            )
            .into());
        }

        // Parse response — same logic as mark_single_student
        let gemini_resp: GeminiResponse = serde_json::from_str(&text).map_err(|e| {
            tracing::error!(adm = adm, parse_error = %e, raw_response = %text, "gemini: failed to parse GeminiResponse (cached)");
            e
        })?;

        let result_text = gemini_resp
            .candidates
            .first()
            .and_then(|c| c.content.parts.iter().find(|p| !p.thought && p.text.is_some()))
            .and_then(|p| p.text.as_ref())
            .ok_or_else(|| {
                tracing::error!(adm = adm, raw_response = %text, "gemini: no text in cached response");
                format!("No text in Gemini cached response for ADM {}", adm)
            })?;

        tracing::debug!(adm = adm, result_json = %result_text, "gemini: raw cached marking JSON");

        let marking: MarkingResult = serde_json::from_str(result_text).map_err(|e| {
            tracing::error!(adm = adm, parse_error = %e, raw_result = %result_text, "gemini: failed to parse cached MarkingResult");
            e
        })?;

        let entry = marking.results.into_iter().next().ok_or_else(|| {
            tracing::error!(adm = adm, "gemini: empty results array (cached)");
            format!("Gemini returned empty results for ADM {} (cached)", adm)
        })?;

        eprintln!(
            "[GEMINI] ADM {} (cached) — score: {}/{} ({} questions)",
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
            "gemini: cached student score summary"
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
                "gemini: cached question breakdown"
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
