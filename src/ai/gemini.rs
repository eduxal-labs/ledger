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
}

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

#[derive(Deserialize)]
struct MarkingResult {
    results: Vec<ScoreEntry>,
}

#[derive(Deserialize)]
struct ScoreEntry {
    adm: i32,
    score: f64,
    total: i32,
}

impl GeminiClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: env!("GEMINI_API_KEY"),
        }
    }

    /// Send marking scheme + student answer sheet URLs to Gemini for grading.
    /// Downloads images from S3 GET URLs, base64-encodes them, and sends as inline_data.
    /// Returns Vec<StudentScore> on success.
    pub async fn mark_paper(
        &self,
        scheme_urls: &[String],
        students: &[(i32, Vec<String>)],
    ) -> Result<Vec<StudentScore>, Box<dyn std::error::Error + Send + Sync>> {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;

        let total_images = scheme_urls.len() + students.iter().map(|(_, u)| u.len()).sum::<usize>();

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

        let dl_start = Instant::now();
        let mut parts = Vec::new();
        parts.push(serde_json::json!({
            "text": "Here is the marking scheme with questions, expected answers, rubric and correct answer examples:"
        }));

        // 1. Download all scheme images
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

        parts.push(serde_json::json!({
            "text": "Now mark the following students' answer sheets."
        }));

        // 2. Download student answer sheet images
        for (adm, urls) in students {
            parts.push(serde_json::json!({ "text": format!("Student ADM {}:", adm) }));
            for (i, url) in urls.iter().enumerate() {
                eprintln!(
                    "[GEMINI] downloading answer sheet for adm={} sheet={}",
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

        // 3. Final instruction
        let adm_list: Vec<String> = students.iter().map(|(adm, _)| adm.to_string()).collect();
        parts.push(serde_json::json!({
            "text": format!(
                "First, determine the total marks for this paper from the marking scheme. Then mark each student's answers against the rubric.

Return ONLY valid JSON:
{{\"results\": [{}]}}",
                adm_list.iter()
                    .map(|a| format!("{{\"adm\": {}, \"score\": <marks_awarded>, \"total\": <total_marks_for_paper>}}", a))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }));

        // 4. Build request body
        let body = serde_json::json!({
            "system_instruction": {
                "parts": [{"text": "You are an expert exam marker for Kenyan secondary school exams. You will be given a marking scheme showing the questions, expected answers, mark allocations, and rubric. First, determine the total marks for the paper by summing all mark allocations in the marking scheme. Then mark each student's answer sheets objectively against the rubric. Award partial marks where the rubric allows. Be precise with mark allocations — every mark must be accounted for."}]
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
            return Err(format!("Gemini API returned status {} — body: {}", status, text).into());
        }

        // 5. Parse Gemini response
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

        let scores: Vec<StudentScore> = marking
            .results
            .into_iter()
            .map(|s| StudentScore {
                adm: s.adm,
                score: s.score,
                total: s.total,
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
