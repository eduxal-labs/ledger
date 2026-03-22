use serde::Deserialize;
use std::time::Instant;

const MODEL: &str = "gemini-3.1-pro-preview";
const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

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

const SYSTEM_INSTRUCTION: &str = r#"You are an expert national-exam marker for Kenyan secondary school examinations (KCSE and equivalent). You mark ALL subjects: Mathematics, Sciences (Biology, Chemistry, Physics), Languages (English, Kiswahili), Humanities (History, Geography, CRE, IRE, HRE), and Technical subjects (Business Studies, Agriculture, Computer Studies, Home Science).

## Your Marking Process

You will receive:
1. MARKING SCHEME images — containing questions, expected answers, mark allocations, and rubric.
2. STUDENT ANSWER SHEET images — one or more pages per student, labelled by their ADM (admission) number.

You must mark in TWO PASSES:

### Pass 1 — Analyse the Marking Scheme
- Read every page of the marking scheme carefully.
- For each question/sub-question, identify: the mark allocation (e.g. M1, A1, B1, or just "1 mark"), the expected answer or acceptable range, and any special rubric instructions.
- Determine the TOTAL marks for the paper by summing all individual mark allocations.
- Note any follow-through (FT) annotations, alternative acceptable answers, or "Accept" notes.

### Pass 2 — Mark Each Student
- For each student, go through EVERY question in the marking scheme order.
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

## Output Requirements

Return your results as a JSON object. For each student, provide:
- Their ADM number
- Their total score
- The total marks for the paper
- A per-question breakdown showing marks awarded, marks available, and a brief note explaining your marking decision

The note for each question should be concise (one sentence) explaining what was awarded and why, especially noting any follow-through applied or marks deducted."#;

// -- Implementation --

impl GeminiClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: env!("GEMINI_API_KEY"),
        }
    }

    /// Send marking scheme + student answer sheet images to Gemini for grading.
    /// Downloads images from S3 GET URLs, base64-encodes them, and sends as inline_data.
    /// Returns Vec<StudentScore> on success, with per-question breakdown for logging.
    pub async fn mark_paper(
        &self,
        scheme_urls: &[String],
        students: &[(i32, Vec<String>)],
    ) -> Result<Vec<StudentScore>, Box<dyn std::error::Error + Send + Sync>> {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;

        let total_images =
            scheme_urls.len() + students.iter().map(|(_, u)| u.len()).sum::<usize>();

        eprintln!(
            "[GEMINI] starting: model={} scheme_images={} students={} total_images={}",
            MODEL,
            scheme_urls.len(),
            students.len(),
            total_images,
        );
        tracing::info!(
            model = MODEL,
            scheme_image_count = scheme_urls.len(),
            student_count = students.len(),
            total_image_count = total_images,
            "gemini: starting mark_paper"
        );

        // -- Download images and build content parts --

        let dl_start = Instant::now();
        let mut parts = Vec::new();

        // Intro text
        parts.push(serde_json::json!({
            "text": "## MARKING SCHEME\n\nThe following images contain the marking scheme for this paper. Study them carefully to identify every question, sub-question, mark allocation, expected answer, and any rubric notes (such as FT, Accept, OR, etc.). Determine the total marks for the paper by summing all mark allocations."
        }));

        // Scheme images
        for (i, url) in scheme_urls.iter().enumerate() {
            eprintln!(
                "[GEMINI] downloading scheme image {}/{}",
                i + 1,
                scheme_urls.len()
            );
            tracing::debug!(index = i, "gemini: downloading scheme image");
            let bytes = self.http.get(url).send().await?.bytes().await?;
            tracing::debug!(
                index = i,
                bytes = bytes.len(),
                "gemini: scheme image downloaded"
            );
            let b64 = STANDARD.encode(&bytes);
            parts.push(serde_json::json!({
                "inline_data": { "mime_type": "image/jpeg", "data": b64 }
            }));
        }

        // Transition text
        parts.push(serde_json::json!({
            "text": "## STUDENT ANSWER SHEETS\n\nNow mark each student's answer sheets against the marking scheme above. Each student is identified by their ADM (admission) number. Apply all marking rules from your instructions — especially follow-through marking."
        }));

        // Student answer sheets
        for (adm, urls) in students {
            parts.push(serde_json::json!({
                "text": format!("### Student ADM {}:", adm)
            }));
            for (i, url) in urls.iter().enumerate() {
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
                let bytes = self.http.get(url).send().await?.bytes().await?;
                tracing::debug!(
                    adm = adm,
                    sheet = i,
                    bytes = bytes.len(),
                    "gemini: student sheet downloaded"
                );
                let b64 = STANDARD.encode(&bytes);
                parts.push(serde_json::json!({
                    "inline_data": { "mime_type": "image/jpeg", "data": b64 }
                }));
            }
        }

        eprintln!(
            "[GEMINI] all {} images downloaded in {}ms — building request body",
            total_images,
            dl_start.elapsed().as_millis()
        );
        tracing::info!(
            total_images = total_images,
            elapsed_ms = dl_start.elapsed().as_millis(),
            "gemini: all images downloaded — building request"
        );

        // -- Final instruction with JSON schema --

        let adm_example: String = students
            .iter()
            .map(|(adm, _)| adm.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let final_instruction = format!(
            r#"## FINAL INSTRUCTIONS

Now produce your marking results. Remember:
- Apply follow-through (FT) marking: only deduct at the point of error, not at every subsequent step.
- Accept equivalent forms unless the rubric explicitly requires a specific form.
- Give benefit of the doubt on ambiguous handwriting.
- The total marks must be determined from the marking scheme (sum of all mark allocations).

The student ADM numbers to include in your results: {}

Return ONLY valid JSON in this exact schema:

{{"results": [{{"adm": <integer>, "score": <number>, "total": <integer>, "breakdown": [{{"q": "<question number, e.g. 1, 2a, 3bi>", "awarded": <number>, "out_of": <number>, "note": "<one-sentence justification>"}}]}}]}}

Every question in the marking scheme must appear in the breakdown for every student. The sum of all "awarded" values must equal the student's "score". The sum of all "out_of" values must equal "total"."#,
            adm_example
        );

        parts.push(serde_json::json!({ "text": final_instruction }));

        // -- Build request body --

        let body = serde_json::json!({
            "system_instruction": {
                "parts": [{"text": SYSTEM_INSTRUCTION}]
            },
            "contents": [{"parts": parts}],
            "generationConfig": {"responseMimeType": "application/json"}
        });

        let request_size = body.to_string().len();
        let url = format!(
            "{}/{}:generateContent?key={}",
            BASE_URL, MODEL, self.api_key
        );

        eprintln!(
            "[GEMINI] sending POST to Gemini API ({} bytes)",
            request_size
        );
        tracing::info!(
            model = MODEL,
            request_body_bytes = request_size,
            "gemini: sending POST to Gemini API"
        );

        // -- Send request --

        let post_start = Instant::now();
        let response = self.http.post(&url).json(&body).send().await?;
        let status = response.status();
        let text = response.text().await?;

        eprintln!(
            "[GEMINI] response received: HTTP {} ({} bytes) in {}ms",
            status.as_u16(),
            text.len(),
            post_start.elapsed().as_millis()
        );
        tracing::info!(
            model = MODEL,
            http_status = status.as_u16(),
            response_bytes = text.len(),
            elapsed_ms = post_start.elapsed().as_millis(),
            "gemini: received response from Gemini API"
        );

        if !status.is_success() {
            eprintln!("[GEMINI] ERROR: HTTP {} — {}", status.as_u16(), text);
            tracing::error!(
                model = MODEL,
                http_status = status.as_u16(),
                response_body = %text,
                "gemini: API returned error status"
            );
            return Err(
                format!("Gemini API returned status {} — body: {}", status, text).into(),
            );
        }

        // -- Parse response --

        let gemini_resp: GeminiResponse = serde_json::from_str(&text).map_err(|e| {
            tracing::error!(
                parse_error = %e,
                raw_response = %text,
                "gemini: failed to parse GeminiResponse wrapper"
            );
            e
        })?;

        let result_text = gemini_resp
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| p.text.as_ref())
            .ok_or_else(|| {
                tracing::error!(raw_response = %text, "gemini: no text found in candidate parts");
                "No text in Gemini response"
            })?;

        tracing::debug!(result_json = %result_text, "gemini: raw marking result JSON");

        let marking: MarkingResult = serde_json::from_str(result_text).map_err(|e| {
            tracing::error!(
                parse_error = %e,
                raw_result = %result_text,
                "gemini: failed to parse MarkingResult from candidate text"
            );
            e
        })?;

        // -- Build scores and log breakdowns --

        let scores: Vec<StudentScore> = marking
            .results
            .into_iter()
            .map(|s| {
                // Log the per-question breakdown for debugging/auditing
                eprintln!(
                    "[GEMINI] ADM {} — score: {}/{} ({} questions in breakdown)",
                    s.adm, s.score, s.total, s.breakdown.len()
                );
                tracing::info!(
                    adm = s.adm,
                    score = s.score,
                    total = s.total,
                    question_count = s.breakdown.len(),
                    "gemini: student score summary"
                );

                for entry in &s.breakdown {
                    let q_label = match &entry.q {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        other => other.to_string(),
                    };
                    eprintln!(
                        "[GEMINI]   Q{}: {}/{} — {}",
                        q_label, entry.awarded, entry.out_of, entry.note
                    );
                    tracing::info!(
                        adm = s.adm,
                        question = %q_label,
                        awarded = entry.awarded,
                        out_of = entry.out_of,
                        note = %entry.note,
                        "gemini: question breakdown"
                    );
                }

                let breakdown = s
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

                StudentScore {
                    adm: s.adm,
                    score: s.score,
                    total: s.total,
                    breakdown,
                }
            })
            .collect();

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
}
