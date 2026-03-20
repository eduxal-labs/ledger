use serde::Deserialize;

const URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:generateContent";

#[derive(Clone)]
pub struct GeminiClient {
    http: reqwest::Client,
    api_key: &'static str,
}

#[derive(Debug)]
pub struct StudentScore {
    pub adm: i32,
    pub score: f64,
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
        total_marks: i32,
    ) -> Result<Vec<StudentScore>, Box<dyn std::error::Error + Send + Sync>> {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;

        // 1. Download all scheme images and base64-encode them
        let mut parts = Vec::new();
        parts.push(serde_json::json!({
            "text": "Here is the marking scheme with questions, expected answers, rubric and correct answer examples:"
        }));

        for url in scheme_urls {
            let bytes = self.http.get(url).send().await?.bytes().await?;
            let b64 = STANDARD.encode(&bytes);
            parts.push(serde_json::json!({
                "inline_data": {
                    "mime_type": "image/jpeg",
                    "data": b64
                }
            }));
        }

        parts.push(serde_json::json!({
            "text": "Now mark the following students' answer sheets."
        }));

        // 2. Download student answer sheet images
        for (adm, urls) in students {
            parts.push(serde_json::json!({
                "text": format!("Student ADM {}:", adm)
            }));
            for url in urls {
                let bytes = self.http.get(url).send().await?.bytes().await?;
                let b64 = STANDARD.encode(&bytes);
                parts.push(serde_json::json!({
                    "inline_data": {
                        "mime_type": "image/jpeg",
                        "data": b64
                    }
                }));
            }
        }

        // 3. Add final instruction with total marks and expected output format
        let adm_list: Vec<String> = students.iter().map(|(adm, _)| adm.to_string()).collect();
        parts.push(serde_json::json!({
            "text": format!(
                "Total marks for this paper: {}\n\nReturn ONLY valid JSON:\n{{\"results\": [{}]}}",
                total_marks,
                adm_list
                    .iter()
                    .map(|a| format!("{{\"adm\": {}, \"score\": <marks>}}", a))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }));

        // 4. Build the full request body
        let body = serde_json::json!({
            "system_instruction": {
                "parts": [{"text": "You are an expert exam marker for Kenyan secondary school exams. Mark objectively and fairly. Award partial marks where the rubric allows."}]
            },
            "contents": [{
                "parts": parts
            }],
            "generationConfig": {
                "responseMimeType": "application/json"
            }
        });

        // 5. POST to Gemini API
        let response = self
            .http
            .post(format!("{}?key={}", URL, self.api_key))
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            tracing::error!("Gemini API error ({}): {}", status, text);
            return Err(format!("Gemini API returned status {}", status).into());
        }

        // 6. Parse Gemini response
        let gemini_resp: GeminiResponse = serde_json::from_str(&text).map_err(|e| {
            tracing::error!("Failed to parse Gemini response: {}. Raw: {}", e, text);
            e
        })?;

        let result_text = gemini_resp
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| p.text.as_ref())
            .ok_or_else(|| {
                tracing::error!("No text in Gemini response: {}", text);
                "No text in Gemini response"
            })?;

        let marking: MarkingResult = serde_json::from_str(result_text).map_err(|e| {
            tracing::error!(
                "Failed to parse marking results: {}. Raw: {}",
                e,
                result_text
            );
            e
        })?;

        Ok(marking
            .results
            .into_iter()
            .map(|s| StudentScore {
                adm: s.adm,
                score: s.score,
            })
            .collect())
    }
}
