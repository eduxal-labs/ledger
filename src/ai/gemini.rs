use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

const MODEL: &str = "gemma-4-31b-it";
const FALLBACK_MODEL: &str = "gemini-2.5-flash-lite";

/// Max concurrent Gemini API requests when marking multiple students.
const MAX_CONCURRENT: usize = 4;

#[derive(Clone, Copy, PartialEq)]
enum ApiProvider {
    Vertex,
    Studio,
}

#[derive(Clone)]
pub struct GeminiClient {
    http: reqwest::Client,
    project_id: &'static str,
    location: &'static str,
    api_provider: ApiProvider,
}

#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub thinking_tokens: i32,
    pub cached_tokens: i32,
    pub total_tokens: i32,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StudentScore {
    pub adm: i32,
    pub score: f64,
    pub total: i32,
    pub breakdown: Vec<QuestionBreakdown>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
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
    #[serde(default)]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Deserialize)]
struct UsageMetadata {
    #[serde(default)]
    prompt_token_count: i32,
    #[serde(default)]
    candidates_token_count: i32,
    #[serde(default)]
    thoughts_token_count: i32,
    #[serde(default, rename = "cachedContentTokenCount")]
    cached_content_token_count: i32,
    #[serde(default)]
    total_token_count: i32,
}

impl UsageMetadata {
    fn to_token_usage(&self) -> TokenUsage {
        TokenUsage {
            input_tokens: self.prompt_token_count,
            output_tokens: self.candidates_token_count,
            thinking_tokens: self.thoughts_token_count,
            cached_tokens: self.cached_content_token_count,
            total_tokens: self.total_token_count,
        }
    }
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

/// Result of marking a single question for a single student.
#[derive(Debug, Clone)]
pub struct QuestionScore {
    pub score: f64,
    pub feedback: String,
}

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

// -- Batch API types --

/// Status of a batch marking job
#[derive(Debug, Clone)]
pub enum BatchStatus {
    Pending,
    Running,
    Succeeded(Vec<BatchStudentResult>),
    Failed(String),
    Cancelled,
    Expired,
}

/// Result for one student within a batch
#[derive(Debug, Clone)]
pub enum BatchStudentResult {
    Ok(StudentScore),
    Err { adm_key: String, error: String },
}

#[derive(Deserialize)]
struct BatchStatusResponse {
    #[allow(dead_code)]
    done: Option<bool>,
    metadata: Option<BatchMetadata>,
    response: Option<BatchResponsePayload>,
    error: Option<BatchError>,
}

#[derive(Deserialize)]
struct BatchMetadata {
    state: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchResponsePayload {
    inlined_responses: Option<Vec<InlinedResponse>>,
}

#[derive(Deserialize)]
struct InlinedResponse {
    response: Option<GeminiResponse>,
    error: Option<BatchError>,
    key: Option<String>,
}

#[derive(Deserialize)]
struct BatchError {
    message: Option<String>,
}

const SYSTEM_INSTRUCTION: &str = r#"You are an expert national-exam marker for Kenyan secondary school examinations (KCSE and equivalent). You mark ALL subjects: Mathematics, Sciences (Biology, Chemistry, Physics), Languages (English, Kiswahili), Humanities (History, Geography, CRE, IRE, HRE), and Technical subjects (Business Studies, Agriculture, Computer Studies, Home Science).

You will receive a marking scheme and ONE student's answer sheets. Mark ONLY this student.

## Your Marking Process

### Step 1 — Analyse the Marking Scheme
- Read every page of the marking scheme carefully.
- For each question/sub-question, identify: the mark allocation (e.g. M1, A1, B1, or just "1 mark"), the expected answer or acceptable range, and the rubric criteria.
- **The rubric criteria are your PRIMARY scoring tool.** Each criterion tells you exactly what to look for in the student's answer. Treat them as your checklist.
- Determine the TOTAL marks for the paper by summing all individual question mark allocations.
- Note any follow-through (FT) annotations, alternative acceptable answers ("Accept", "OR"), or required qualifiers ("must mention X").

### Step 2 — Mark the Student Using the Rubric
- Go through EVERY question in the marking scheme order.
- **For each question, check the rubric criteria FIRST** before comparing against the model answer. The rubric tells you what specific elements earn marks.
- Match each rubric criterion against the student's answer:
  - **Full match**: the student's answer contains the exact concept/fact/step the criterion demands → award full marks for that criterion.
  - **Partial match**: the student's answer is on the right track but incomplete or imprecise → award partial marks proportional to how much of the criterion was satisfied.
  - **No match**: the criterion is not addressed at all → award 0 for that criterion.
  - **OR criteria**: if the rubric says "Accept X OR Y", award the criterion if the student gives either.
- Sum the awarded criterion marks to get the question score. **CAP the question score at the question's own allocated mark total** — rubric criteria marks are scoring guides, and their sum may exceed the question's maximum to provide marking flexibility.
- Record a brief justification per question stating which criteria were met, which were missed, and why any marks were deducted.

## Understanding Rubric Criteria Structure

Rubric criteria come in several forms. Recognise them:

1. **Point-by-point criteria** — each criterion is a distinct mark point. Award each independently based on whether the student's answer contains it.
2. **Band descriptors** — criteria organised in bands (e.g. 0-2, 3-4, 5-6 marks). Place the student in the band that best matches their overall answer quality, then fine-tune within the band.
3. **Negative criteria** — criteria that say "deduct 1 mark if missing/incorrect." Apply these as deductions from the question total, not as positive awards.
4. **Compound criteria** — a single criterion that requires multiple elements (e.g. "correct formula AND substitution AND final answer"). Award proportionally — if 2 of 3 elements are present, award ~⅔ of the criterion's marks.

## Mark Types

- **B marks** (standalone): Awarded for a correct result, fact, or statement. No working is required. A B mark is earned if the rubric criterion for that fact/result is satisfied.
- **M marks** (method): Awarded for a correct method, procedure, or approach — even if the numbers used are wrong (provided they come from the student's own earlier working). M marks reward the PROCESS, not the answer. Award the M mark if the rubric criterion describing the method is satisfied.
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

## Partial Credit Guidelines

When a student's answer partially satisfies a rubric criterion:
- **Concept present but incomplete**: award 50-70% of the criterion's marks.
- **Correct approach but execution flawed**: award the method portion, deduct the accuracy portion.
- **Multiple criteria, some met and some not**: award only the met criteria; do not average across all criteria.
- **Vague or imprecise language that still conveys the idea**: award 50% of the criterion's marks.
- **Correct answer with no shown working**: award accuracy marks but NOT method marks (unless the rubric says working is not required).

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
- Use the rubric's band descriptors as your primary reference — place the answer in the band that best fits, then fine-tune.
- Do not penalise for dialect variations that are acceptable in the Kenyan curriculum context.

## Rubric Marks vs Question Marks (CRITICAL)

Rubric criteria marks are SCORING GUIDES, not a cap on what a student can earn. They may exceed a question's total allocated marks to provide variety in acceptable answers. The MAXIMUM marks a student can earn for any question is the question's own allocated marks — NOT the sum of rubric criteria marks.

Marking algorithm per question:
1. Go through every rubric criterion. Award marks for each criterion the student satisfies.
2. Sum the awarded criterion marks.
3. Cap the total at the question's allocated marks (a student cannot exceed the question max).
4. This means: if a question is worth 3 marks and the rubric has 8 criteria totalling 15 marks, the student gets min(sum of met criteria, 3).

Example: A question worth 3 marks may have rubric criteria listing 8 possible mark points. A student who hits 3 of those points earns the full 3 marks. A student who hits only 2 criteria gets 2 marks. A student who somehow hits all 8 criteria still gets only 3 marks (the question cap).

## Output Format

Return ONLY a JSON object with exactly one entry in the results array (for the single student you are marking):

{"results": [{"adm": <integer>, "score": <number>, "total": <integer>, "breakdown": [{"q": "<question number>", "awarded": <number>, "out_of": <number>, "note": "<one-sentence justification: which rubric criteria were met, which were missed, follow-through or deductions applied>"}]}]}

Rules:
- Every question in the marking scheme MUST appear in the breakdown.
- The sum of all "awarded" values MUST equal "score".
- The sum of all "out_of" values MUST equal "total".
- The "note" must reference rubric criteria explicitly (e.g. "Met criteria 1-3 (definition, formula, substitution), missed criterion 4 (wrong final answer) → 3/4")."#;

// -- Implementation --

impl GeminiClient {
    pub fn new() -> Self {
        use reqwest::header::{HeaderMap, HeaderValue};

        // Detect API provider: "vertex" → Vertex AI, anything else → AI Studio.
        // GEMINI_API_PROVIDER is a compile-time env var set via build.rs from .env.
        let api_provider = match option_env!("GEMINI_API_PROVIDER") {
            Some("vertex") => ApiProvider::Vertex,
            _ => ApiProvider::Studio,
        };

        let api_key = match api_provider {
            ApiProvider::Vertex => env!("GEMINI_API_KEY"),
            ApiProvider::Studio => env!("GEMINI_STUDIO_API_KEY"),
        };

        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            "x-goog-api-key",
            HeaderValue::from_static(api_key),
        );
        Self {
            http: reqwest::Client::builder()
                .default_headers(default_headers)
                .build()
                .expect("failed to build reqwest client"),
            project_id: option_env!("GEMINI_PROJECT_ID").unwrap_or(""),
            location: option_env!("GEMINI_LOCATION").unwrap_or(""),
            api_provider,
        }
    }

    /// Generate-content endpoint for the given model.
    fn generate_content_url(&self, model: &str) -> String {
        match self.api_provider {
            ApiProvider::Vertex => format!(
                "https://aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
                self.project_id, self.location, model
            ),
            ApiProvider::Studio => format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
                model
            ),
        }
    }

    /// CachedContents collection endpoint.
    fn cache_base_url(&self) -> String {
        match self.api_provider {
            ApiProvider::Vertex => format!(
                "https://aiplatform.googleapis.com/v1/projects/{}/locations/{}/cachedContents",
                self.project_id, self.location
            ),
            ApiProvider::Studio => "https://generativelanguage.googleapis.com/v1beta/cachedContents".to_string(),
        }
    }

    /// URL for a specific resource (cachedContent or batch job) by its full resource name.
    fn resource_url(&self, resource_name: &str) -> String {
        match self.api_provider {
            ApiProvider::Vertex => format!("https://aiplatform.googleapis.com/v1/{}", resource_name),
            ApiProvider::Studio => format!("https://generativelanguage.googleapis.com/v1beta/{}", resource_name),
        }
    }

    /// Full model resource name (used in cache creation body).
    fn model_resource_name(&self, model: &str) -> String {
        match self.api_provider {
            ApiProvider::Vertex => format!(
                "projects/{}/locations/{}/publishers/google/models/{}",
                self.project_id, self.location, model
            ),
            ApiProvider::Studio => format!("models/{}", model),
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
            "text": "## MARKING SCHEME\n\nThe following images contain the marking scheme for this paper. Study them carefully to identify every question, sub-question, mark allocation, rubric criterion, expected answer, and any rubric notes (such as FT, Accept, OR, etc.). The rubric criteria are your PRIMARY scoring tool — they tell you exactly what to look for in the student's answer. Determine the total marks for the paper by summing all QUESTION mark allocations. Note: rubric criteria marks are scoring guides that may exceed a question's allocated marks to provide flexibility — always cap the awarded marks at the question's own mark allocation."
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

    /// Build the content parts for a single student's marking request.
    /// Used by both mark_student_cached (real-time) and create_batch_job (batch).
    fn build_student_parts(adm: i32, answer_images_b64: &[String]) -> Vec<serde_json::Value> {
        let mut parts = Vec::with_capacity(answer_images_b64.len() + 2);

        parts.push(serde_json::json!({
            "text": format!("## STUDENT ANSWER SHEETS\n\nMark the following student's answer sheets against the marking scheme above. Apply all marking rules — especially follow-through marking.\n\n### Student ADM {}:", adm)
        }));

        for b64 in answer_images_b64 {
            parts.push(serde_json::json!({
                "inline_data": { "mime_type": "image/jpeg", "data": b64 }
            }));
        }

        let final_instruction = format!(
            r#"## FINAL INSTRUCTIONS

Mark student ADM {} against the marking scheme above.

Remember:
- Use the rubric criteria as your PRIMARY checklist for each question.
- Match each criterion against the student's answer: full match → full marks for that criterion; partial match → proportional marks; no match → 0.
- Sum the met criteria marks, then CAP at the question's own allocated marks.
- Apply follow-through (FT) marking: only deduct at the point of error, not at every subsequent step.
- Accept equivalent forms unless the rubric explicitly requires a specific form.
- Give benefit of the doubt on ambiguous handwriting.

Return ONLY valid JSON with exactly one result entry for this student:

{{"results": [{{"adm": {}, "score": <marks_awarded>, "total": <total_marks_for_paper>, "breakdown": [{{"q": "<question number>", "awarded": <number>, "out_of": <number>, "note": "<which rubric criteria were met/missed, and why>"}}]}}]}}"#,
            adm, adm
        );
        parts.push(serde_json::json!({ "text": final_instruction }));

        parts
    }

    /// Build content parts for a context cache from paper questions and rubrics.
    ///
    /// Each entry is `(question_num, question_text, marks, rubric_text, image_b64s)`.
    /// The system instruction is added separately via `systemInstruction` in the
    /// cache creation request, so this function only produces the user-facing
    /// marking-scheme parts.
    pub fn build_question_cache_parts(
        questions: &[(i32, &str, i16, &str, &[(String, Option<String>)])],
    ) -> Vec<serde_json::Value> {
        let mut parts: Vec<serde_json::Value> = Vec::new();

        // Header part — describes the scheme
        parts.push(serde_json::json!({
            "text": "## MARKING SCHEME\n\nThe following is the complete marking scheme for this paper. It lists every question with its text, total marks, and rubric criteria. Rubric criteria are your PRIMARY scoring tool — each one tells you exactly what to look for in the student's answer.\n\nMarking rules:\n- Match each rubric criterion against the student's answer: full match → full criterion marks; partial match → proportional marks; no match → 0.\n- Sum awarded criterion marks, then CAP at the question's own allocated marks.\n- Apply follow-through (FT) marking: only deduct at the point of error.\n- Accept equivalent forms unless the rubric explicitly requires a specific form.\n- Give benefit of the doubt on ambiguous handwriting."
        }));

        // One part per question — structured text
        for (q_num, q_text, marks, rubric, images) in questions {
            let mut text = format!(
                "QUESTION {} ({} marks)\n\n{}\n\nRUBRIC:\n{}\n---",
                q_num, marks, q_text, rubric
            );
            parts.push(serde_json::json!({ "text": text }));

            // Inline question images (diagrams, passages, etc.)
            for (b64, caption) in *images {
                parts.push(serde_json::json!({
                    "inline_data": { "mime_type": "image/jpeg", "data": b64 }
                }));
                if let Some(cap) = caption {
                    parts.push(serde_json::json!({
                        "text": format!("Image caption: {}", cap)
                    }));
                }
            }
        }

        parts
    }

    /// Send a generateContent request, retrying once with FALLBACK_MODEL on HTTP 503.
    async fn send_with_fallback(
        &self,
        body: &serde_json::Value,
        label: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let request_size = body.to_string().len();
        let url = self.generate_content_url(MODEL);

        eprintln!("[GEMINI] {}: sending POST ({} bytes)", label, request_size);
        tracing::info!(
            model = MODEL,
            label = %label,
            request_body_bytes = request_size,
            "gemini: sending POST"
        );

        let post_start = Instant::now();
        let response = self.http.post(&url).json(body).send().await?;
        let status = response.status();
        let text = response.text().await?;

        eprintln!(
            "[GEMINI] {}: response HTTP {} ({} bytes) in {}ms",
            label,
            status.as_u16(),
            text.len(),
            post_start.elapsed().as_millis()
        );
        tracing::info!(
            model = MODEL,
            label = %label,
            http_status = status.as_u16(),
            response_bytes = text.len(),
            elapsed_ms = post_start.elapsed().as_millis(),
            "gemini: received response"
        );

        if status.as_u16() == 503 {
            eprintln!(
                "[GEMINI] {}: HTTP 503 — retrying with fallback model {}",
                label, FALLBACK_MODEL
            );
            tracing::warn!(
                label = %label,
                fallback_model = FALLBACK_MODEL,
                "gemini: HTTP 503 — retrying with fallback model"
            );

            let fallback_url = self.generate_content_url(FALLBACK_MODEL);
            let fallback_start = Instant::now();
            let response = self.http.post(&fallback_url).json(body).send().await?;
            let status = response.status();
            let text = response.text().await?;

            eprintln!(
                "[GEMINI] {} (fallback): response HTTP {} ({} bytes) in {}ms",
                label,
                status.as_u16(),
                text.len(),
                fallback_start.elapsed().as_millis()
            );
            tracing::info!(
                model = FALLBACK_MODEL,
                label = %label,
                http_status = status.as_u16(),
                response_bytes = text.len(),
                elapsed_ms = fallback_start.elapsed().as_millis(),
                "gemini: received fallback response"
            );

            if !status.is_success() {
                eprintln!(
                    "[GEMINI] {} (fallback): ERROR HTTP {} — {}",
                    label,
                    status.as_u16(),
                    text
                );
                tracing::error!(
                    model = FALLBACK_MODEL,
                    label = %label,
                    http_status = status.as_u16(),
                    response_body = %text,
                    "gemini: fallback API error"
                );
                return Err(format!(
                    "Gemini fallback API returned status {} for {} — body: {}",
                    status, label, text
                )
                .into());
            }

            return Ok(text);
        }

        if !status.is_success() {
            eprintln!(
                "[GEMINI] {}: ERROR HTTP {} — {}",
                label,
                status.as_u16(),
                text
            );
            tracing::error!(
                model = MODEL,
                label = %label,
                http_status = status.as_u16(),
                response_body = %text,
                "gemini: API error"
            );
            return Err(format!(
                "Gemini API returned status {} for {} — body: {}",
                status, label, text
            )
            .into());
        }

        Ok(text)
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
- Use the rubric criteria as your PRIMARY checklist for each question.
- Match each criterion against the student's answer: full match → full marks for that criterion; partial match → proportional marks; no match → 0.
- Sum the met criteria marks, then CAP at the question's own allocated marks.
- Apply follow-through (FT) marking: only deduct at the point of error, not at every subsequent step.
- Accept equivalent forms unless the rubric explicitly requires a specific form.
- Give benefit of the doubt on ambiguous handwriting.

Return ONLY valid JSON with exactly one result entry for this student:

{{"results": [{{"adm": {}, "score": <marks_awarded>, "total": <total_marks_for_paper>, "breakdown": [{{"q": "<question number>", "awarded": <number>, "out_of": <number>, "note": "<which rubric criteria were met/missed, and why>"}}]}}]}}"#,
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
                    "thinkingLevel": "medium"
                }
            }
        });

        let label = format!("ADM {}", adm);
        let text = self.send_with_fallback(&body, &label).await?;

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
            usage: None,
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
            student_count = 0,
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
            student_count = 0,
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
        let url = self.cache_base_url();

        let body = serde_json::json!({
            "model": self.model_resource_name(MODEL),
            "systemInstruction": {
                "parts": [{"text": SYSTEM_INSTRUCTION}]
            },
            "contents": [{"parts": scheme_parts, "role": "user"}],
            "ttl": "3600s"
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
        let url = self.resource_url(cache_name);

        eprintln!("[GEMINI] deleting context cache: {}", cache_name);
        tracing::info!(cache_name = %cache_name, "gemini: deleting context cache");

        match self.http.delete(&url).send().await {
            Ok(resp) => {
                eprintln!("[GEMINI] cache delete: HTTP {}", resp.status().as_u16());
                tracing::info!(
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

    /// Mark a single student WITHOUT a context cache (legacy path).
    /// Sends the full system instruction + student answer images in one request.
    /// Used for papers that don't have structured question data.
    pub async fn mark_student_uncached(
        &self,
        adm: i32,
        answer_images_b64: &[String],
    ) -> Result<StudentScore, Box<dyn std::error::Error + Send + Sync>> {
        let parts = Self::build_student_parts(adm, answer_images_b64);

        let body = serde_json::json!({
            "system_instruction": {
                "parts": [{"text": SYSTEM_INSTRUCTION}]
            },
            "contents": [{"parts": parts, "role": "user"}],
            "generationConfig": {
                "responseMimeType": "application/json",
                "temperature": 0,
                "thinkingConfig": {
                    "thinkingLevel": "medium"
                }
            }
        });

        let label = format!("ADM {} (uncached)", adm);
        let text = self.send_with_fallback(&body, &label).await?;

        let gemini_resp: GeminiResponse = serde_json::from_str(&text).map_err(|e| {
            tracing::error!(adm = adm, parse_error = %e, raw_response = %text, "gemini: failed to parse GeminiResponse (uncached)");
            e
        })?;

        let usage = gemini_resp.usage_metadata.as_ref().map(|m| m.to_token_usage());

        let result_text = gemini_resp
            .candidates
            .first()
            .and_then(|c| c.content.parts.iter().find(|p| !p.thought && p.text.is_some()))
            .and_then(|p| p.text.as_ref())
            .ok_or_else(|| {
                tracing::error!(adm = adm, raw_response = %text, "gemini: no text in uncached response");
                format!("No text in Gemini uncached response for ADM {}", adm)
            })?;

        let marking: MarkingResult = serde_json::from_str(result_text).map_err(|e| {
            tracing::error!(adm = adm, parse_error = %e, raw_result = %result_text, "gemini: failed to parse uncached MarkingResult");
            e
        })?;

        let entry = marking.results.into_iter().next().ok_or_else(|| {
            tracing::error!(adm = adm, "gemini: empty results array (uncached)");
            format!("Gemini returned empty results for ADM {} (uncached)", adm)
        })?;

        tracing::info!(
            adm = entry.adm,
            score = entry.score,
            total = entry.total,
            "gemini: uncached student score summary"
        );

        let breakdown = entry
            .breakdown
            .into_iter()
            .map(|b| {
                let question = match b.q {
                    serde_json::Value::String(s) => s,
                    serde_json::Value::Number(n) => n.to_string(),
                    other => other.to_string(),
                };
                QuestionBreakdown { question, awarded: b.awarded, out_of: b.out_of, note: b.note }
            })
            .collect();

        Ok(StudentScore {
            adm: entry.adm,
            score: entry.score,
            total: entry.total,
            breakdown,
            usage,
        })
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
        let parts = Self::build_student_parts(adm, answer_images_b64);

        // Build request using cachedContent — no system_instruction, no scheme parts
        let body = serde_json::json!({
            "cachedContent": cache_name,
            "contents": [{"parts": parts, "role": "user"}],
            "generationConfig": {
                "responseMimeType": "application/json",
                "temperature": 0,
                "thinkingConfig": {
                    "thinkingLevel": "medium"
                }
            }
        });

        let request_size = body.to_string().len();
        let url = self.generate_content_url(MODEL);

        eprintln!(
            "[GEMINI] ADM {} (cached): sending POST ({} bytes)",
            adm, request_size
        );
        tracing::info!(
            model = MODEL,
            adm = adm,
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

        let usage = gemini_resp.usage_metadata.as_ref().map(|m| m.to_token_usage());

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
            usage,
        })
    }

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
        student_images_b64: &[String],
        questions: &[AllQuestionInput],
    ) -> std::result::Result<Vec<(i32, QuestionScore)>, Box<dyn std::error::Error + Send + Sync>>
    {
        let start = std::time::Instant::now();

        // Step A — Build parts starting with student answer sheet images
        let mut parts: Vec<serde_json::Value> = Vec::new();

        for img in student_images_b64 {
            parts.push(serde_json::json!({
                "inline_data": { "mime_type": "image/jpeg", "data": img }
            }));
        }

        // Step B — For each question, push a structured text block followed by any question images
        for (i, q) in questions.iter().enumerate() {
            let rubric_string = q
                .rubric
                .iter()
                .enumerate()
                .map(|(j, (criterion, marks))| {
                    format!("{}. {} ({} marks)", j + 1, criterion, marks)
                })
                .collect::<Vec<_>>()
                .join("\n");

            parts.push(serde_json::json!({
                "text": format!(
                    "QUESTION {} (ID: {}, Total: {} marks):\n{}\n\nRubric:\n{}\n---",
                    i + 1,
                    q.question_id,
                    q.marks,
                    q.text,
                    rubric_string
                )
            }));

            for (img_b64, caption) in &q.images_b64 {
                parts.push(serde_json::json!({
                    "inline_data": { "mime_type": "image/jpeg", "data": img_b64 }
                }));
                if let Some(cap) = caption {
                    parts.push(serde_json::json!({
                        "text": format!("Image caption: {}", cap)
                    }));
                }
            }
        }

        // Step C — Push the final instruction text part
        parts.push(serde_json::json!({
            "text": "Mark every question listed above for this student. Find each question's answer in the student's answer sheets, then score it using its rubric criteria as your primary checklist:\n\n1. Go through each rubric criterion — full match → full marks for that criterion; partial match → proportional marks; no match → 0.\n2. Sum the awarded criterion marks.\n3. CAP at the question's own allocated marks (the question's max, NOT the sum of rubric criteria).\n\nReturn ONLY valid JSON:\n{\"results\": [\n  {\"question_id\": <integer>, \"score\": <number>, \"feedback\": \"<which rubric criteria met/missed and why>\"},\n  ...\n]}\n\nRules:\n- Every question_id listed above MUST appear exactly once in results.\n- score for each question MUST be >= 0 and MUST NOT exceed that question's total marks.\n- Partial credit is mandatory for partially correct answers — never give 0 unless nothing is correct."
        }));

        // Step D — Build the request body (cachedContent pattern)
        let body = serde_json::json!({
            "contents": [{"role": "user", "parts": parts}],
            "generationConfig": {
                "responseMimeType": "application/json",
                "temperature": 0,
                "thinkingConfig": { "thinkingLevel": "medium" }
            }
        });

        let request_size = body.to_string().len();
        eprintln!(
            "[Gemini] mark_all_questions: request_size={} bytes, questions={}",
            request_size,
            questions.len()
        );

        // Step E — Send, log, and parse
        let url = self.generate_content_url(MODEL);
        let response = self.http.post(&url).json(&body).send().await?;

        let status = response.status();
        let text = response.text().await?;
        let elapsed = start.elapsed().as_millis();
        let response_size = text.len();

        eprintln!(
            "[Gemini] mark_all_questions: status={}, response_size={} bytes, elapsed={}ms",
            status, response_size, elapsed
        );

        if !status.is_success() {
            return Err(format!("Gemini API error {}: {}", status, text).into());
        }

        // Parse GeminiResponse → extract first non-thought text part
        let gemini_resp: GeminiResponse = serde_json::from_str(&text)
            .map_err(|e| format!("failed to parse GeminiResponse: {}: {}", e, text))?;

        let extracted = gemini_resp
            .candidates
            .into_iter()
            .next()
            .and_then(|c| {
                c.content
                    .parts
                    .into_iter()
                    .find(|p| !p.thought && p.text.is_some())
            })
            .and_then(|p| p.text)
            .ok_or("no text part found in Gemini response")?;

        let result: AllQuestionsApiResult = serde_json::from_str(&extracted).map_err(|e| {
            format!(
                "failed to parse AllQuestionsApiResult: {}: {}",
                e, extracted
            )
        })?;

        // Log per-question scores
        for entry in &result.results {
            if let Some(q) = questions
                .iter()
                .find(|q| q.question_id == entry.question_id)
            {
                tracing::info!(
                    question_id = entry.question_id,
                    score = entry.score,
                    marks = q.marks,
                    "mark_all_questions: question scored"
                );
            }
        }

        let question_count = result.results.len();
        tracing::info!(
            question_count = question_count,
            total_elapsed_ms = elapsed,
            "mark_all_questions: completed"
        );

        Ok(result
            .results
            .into_iter()
            .map(|e| {
                (
                    e.question_id,
                    QuestionScore {
                        score: e.score,
                        feedback: e.feedback,
                    },
                )
            })
            .collect())
    }

    /// Submit a batch marking job for multiple students.
    /// Each student's request uses the pre-created context cache.
    /// Returns the batch job name (e.g., "batches/123456").
    pub async fn create_batch_job(
        &self,
        students: &[(i32, &[String])],
        display_name: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Vertex AI does not have a batchGenerateContent endpoint. Its batch
        // prediction API (BatchPredictionJob) requires GCS or BigQuery as
        // input/output and does not support explicit context caching. Returning
        // an error here causes mark_batch_with_fallback to transparently fall
        // back to the concurrent real-time path (mark_paper / mark_student_cached).
        tracing::warn!(
            display_name = %display_name,
            student_count = 0,
            "gemini: batch inference is not supported on Vertex AI \
             (requires GCS/BigQuery input; explicit caching unsupported) — \
             caller should fall back to real-time marking"
        );
        eprintln!(
            "[GEMINI] create_batch_job: Vertex AI batch inference is not supported \
             (no batchGenerateContent endpoint; explicit caching unsupported). \
             Falling back to real-time marking."
        );
        Err(
            "Vertex AI batch inference requires GCS/BigQuery input and does not \
             support explicit context caching. Use mark_paper or mark_student_cached \
             for concurrent real-time marking instead."
                .into(),
        )
    }

    /// Poll the status of a batch job. Returns the current status.
    /// If the job succeeded, includes parsed StudentScore results.
    pub async fn get_batch_status(
        &self,
        batch_name: &str,
    ) -> Result<BatchStatus, Box<dyn std::error::Error + Send + Sync>> {
        let url = self.resource_url(batch_name);

        let response = self.http.get(&url).send().await?;
        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(format!("Batch status check returned HTTP {} — {}", status, text).into());
        }

        let resp: BatchStatusResponse = serde_json::from_str(&text).map_err(|e| {
            tracing::error!(
                parse_error = %e,
                raw_response = %text,
                "gemini: failed to parse BatchStatusResponse"
            );
            e
        })?;

        let state = resp
            .metadata
            .as_ref()
            .map(|m| m.state.as_str())
            .unwrap_or("UNKNOWN");

        match state {
            "JOB_STATE_PENDING" => Ok(BatchStatus::Pending),
            "JOB_STATE_RUNNING" => Ok(BatchStatus::Running),
            "JOB_STATE_CANCELLED" => Ok(BatchStatus::Cancelled),
            "JOB_STATE_EXPIRED" => Ok(BatchStatus::Expired),
            "JOB_STATE_FAILED" => {
                let msg = resp
                    .error
                    .and_then(|e| e.message)
                    .unwrap_or_else(|| "unknown error".to_string());
                Ok(BatchStatus::Failed(msg))
            }
            "JOB_STATE_SUCCEEDED" => {
                let inlined = resp
                    .response
                    .and_then(|r| r.inlined_responses)
                    .unwrap_or_default();

                let mut results = Vec::with_capacity(inlined.len());

                for item in inlined {
                    let adm_key = item.key.unwrap_or_default();
                    let adm: i32 = adm_key
                        .strip_prefix("adm-")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);

                    if let Some(err) = item.error {
                        let msg = err
                            .message
                            .unwrap_or_else(|| "unknown batch item error".to_string());
                        results.push(BatchStudentResult::Err {
                            adm_key,
                            error: msg,
                        });
                        continue;
                    }

                    let gemini_resp = match item.response {
                        Some(r) => r,
                        None => {
                            results.push(BatchStudentResult::Err {
                                adm_key,
                                error: "no response in batch item".to_string(),
                            });
                            continue;
                        }
                    };

                    // Same parsing logic as mark_student_cached
                    let result_text = match gemini_resp
                        .candidates
                        .first()
                        .and_then(|c| {
                            c.content
                                .parts
                                .iter()
                                .find(|p| !p.thought && p.text.is_some())
                        })
                        .and_then(|p| p.text.as_ref())
                    {
                        Some(t) => t,
                        None => {
                            results.push(BatchStudentResult::Err {
                                adm_key,
                                error: "no text in batch response".to_string(),
                            });
                            continue;
                        }
                    };

                    let marking: MarkingResult = match serde_json::from_str(result_text) {
                        Ok(m) => m,
                        Err(e) => {
                            results.push(BatchStudentResult::Err {
                                adm_key,
                                error: format!("JSON parse error: {}", e),
                            });
                            continue;
                        }
                    };

                    let entry = match marking.results.into_iter().next() {
                        Some(e) => e,
                        None => {
                            results.push(BatchStudentResult::Err {
                                adm_key,
                                error: "empty results array".to_string(),
                            });
                            continue;
                        }
                    };

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

                    results.push(BatchStudentResult::Ok(StudentScore {
                        adm: if adm != 0 { adm } else { entry.adm },
                        score: entry.score,
                        total: entry.total,
                        breakdown,
                        usage: None,
                    }));
                }

                Ok(BatchStatus::Succeeded(results))
            }
            other => Ok(BatchStatus::Failed(format!(
                "unknown batch state: {}",
                other
            ))),
        }
    }

    /// Cancel an ongoing batch job (best-effort).
    pub async fn cancel_batch_job(&self, batch_name: &str) {
        let url = format!("{}:cancel", self.resource_url(batch_name));

        eprintln!("[GEMINI] cancelling batch job: {}", batch_name);
        tracing::info!(batch_name = %batch_name, "gemini: cancelling batch job");

        match self.http.post(&url).send().await {
            Ok(resp) => {
                eprintln!("[GEMINI] batch cancel: HTTP {}", resp.status().as_u16());
                tracing::info!(
                    batch_name = %batch_name,
                    http_status = resp.status().as_u16(),
                    "gemini: batch cancel response"
                );
            }
            Err(e) => {
                eprintln!("[GEMINI] batch cancel failed: {}", e);
                tracing::warn!(
                    batch_name = %batch_name,
                    error = %e,
                    "gemini: batch cancel failed (ignored)"
                );
            }
        }
    }

    /// Delete a batch job (best-effort, fire-and-forget).
    pub async fn delete_batch_job(&self, batch_name: &str) {
        let url = self.resource_url(batch_name);

        eprintln!("[GEMINI] deleting batch job: {}", batch_name);
        tracing::info!(batch_name = %batch_name, "gemini: deleting batch job");

        match self.http.delete(&url).send().await {
            Ok(resp) => {
                eprintln!("[GEMINI] batch delete: HTTP {}", resp.status().as_u16());
                tracing::info!(
                    batch_name = %batch_name,
                    http_status = resp.status().as_u16(),
                    "gemini: batch deleted"
                );
            }
            Err(e) => {
                eprintln!("[GEMINI] batch delete failed: {}", e);
                tracing::warn!(
                    batch_name = %batch_name,
                    error = %e,
                    "gemini: batch delete failed (ignored)"
                );
            }
        }
    }
}
