# Ledger — Task Board

## Feature: AI-Powered Exam Marking

### Overview

Add a new `AiMarking` gRPC service that:
1. Issues presigned S3 PUT URLs so clients can upload marking scheme + answer sheet images
2. Accepts a `MarkPaper` request, fetches images from S3, sends them to Gemini for marking, writes grades to DB
3. Grades sync to all clients automatically via the existing `watchChanges` changelog

S3 path convention:
```
schools/{school_id}/exams/{exam_id}/papers/{subject}_{paper}/scheme/{n}
schools/{school_id}/exams/{exam_id}/papers/{subject}_{paper}/students/{adm}/{n}
```
Where `paper` is the paper number as string (use `0` for null/single paper subjects).

### Dependency Graph

```
S1 (Proto) ──► S2 (S3 helpers) ──► S4 (Implement handlers) ──► S5 (Register)
                S3 (Gemini client) ─┘
```

S2 and S3 can run in parallel. S4 depends on both. S5 depends on S4.

---

### Task S1: Create `ai_marking.proto` and update `build.rs`

**Files to create:** `protos/services/ai_marking.proto`
**Files to modify:** `build.rs`
**Depends on:** none
**Parallel group:** —

**Specification:**

**Step 1 — Create `protos/services/ai_marking.proto`:**

```proto
syntax = "proto3";

package ai_marking;

service AiMarking {
  rpc RequestUploadUrls(UploadUrlsRequest) returns (UploadUrlsResponse);
  rpc MarkPaper(MarkPaperRequest) returns (MarkPaperResponse);
}

message UploadUrlsRequest {
  string school = 1;
  string exam = 2;
  int32 subject = 3;
  optional int32 paper = 4;
  int32 scheme_count = 5;
  repeated StudentSheetCount students = 6;
}

message StudentSheetCount {
  int32 adm = 1;
  int32 count = 2;
}

message UploadUrlsResponse {
  repeated SignedUrl scheme_urls = 1;
  repeated StudentSignedUrls student_urls = 2;
}

message SignedUrl {
  string key = 1;
  string url = 2;
}

message StudentSignedUrls {
  int32 adm = 1;
  repeated SignedUrl urls = 2;
}

message MarkPaperRequest {
  string school = 1;
  string exam = 2;
  int32 subject = 3;
  optional int32 paper = 4;
  int32 grade = 5;
  optional int32 stream = 6;
  int32 total_marks = 7;
  repeated string scheme_keys = 8;
  repeated StudentMarkTarget students = 9;
}

message StudentMarkTarget {
  int32 adm = 1;
  repeated string keys = 2;
}

message MarkPaperResponse {
  bool accepted = 1;
  string message = 2;
}
```

**Step 2 — Update `build.rs`:**

Add `"./protos/services/ai_marking.proto"` to the `compile_protos` array. The updated call:

```rust
.compile_protos(
    &[
        "./protos/services/authentication.proto",
        "./protos/services/sync.proto",
        "./protos/services/ai_marking.proto",
        "./protos/types/role.proto",
        "./protos/types/member.proto",
    ],
    &["./protos/"],
)?;
```

**Step 3 — Verify compilation:**

Run `cargo build` and ensure the proto compiles. The generated Rust stubs will be available via `tonic::include_proto!("ai_marking")`.

**Step 4 — Generate Dart stubs for client:**

The client (Flutter) also needs Dart stubs. Run protoc with the Dart plugin:

```bash
cd ../eduxal
protoc --dart_out=grpc:lib/proto/services/ -I../ledger/protos/ services/ai_marking.proto
```

If `protoc-gen-dart` is not installed, install it:
```bash
dart pub global activate protoc_plugin
```

Verify files are generated in `../eduxal/lib/proto/services/`:
- `ai_marking.pb.dart`
- `ai_marking.pbenum.dart`
- `ai_marking.pbgrpc.dart`
- `ai_marking.pbjson.dart`

**Update after completion:**
- [ ] Verify `cargo build` succeeds
- [ ] Verify Dart stubs exist in `../eduxal/lib/proto/services/`
- [ ] Mark this task `[x]`
- [ ] git commit: `feat: add ai_marking.proto with RequestUploadUrls and MarkPaper RPCs`

---

### Task S2: Add S3 path helpers for exam paper files

**Files to modify:** `src/config/storage/sign.rs`
**Depends on:** none
**Parallel group:** P1

**Specification:**

Add two new public helper functions below the existing `logo()` function in `sign.rs`:

```rust
/// Presigned URL for a marking scheme image page.
/// Path: schools/{school}/exams/{exam}/papers/{subject}_{paper}/scheme/{index}
pub fn scheme_image(school: &str, exam: &str, subject: i32, paper: i32, index: i32, ttl: u64, write: bool) -> String {
    let key = format!("schools/{}/exams/{}/papers/{}_{}/scheme/{}", school, exam, subject, paper, index);
    url(&key, ttl, write)
}

/// Presigned URL for a student answer sheet image.
/// Path: schools/{school}/exams/{exam}/papers/{subject}_{paper}/students/{adm}/{index}
pub fn answer_sheet(school: &str, exam: &str, subject: i32, paper: i32, adm: i32, index: i32, ttl: u64, write: bool) -> String {
    let key = format!("schools/{}/exams/{}/papers/{}_{}/students/{}/{}", school, exam, subject, paper, adm, index);
    url(&key, ttl, write)
}
```

These use the existing `url()` function which handles the presigning.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `feat: add S3 path helpers for exam paper scheme and answer sheets`

---

### Task S3: Create Gemini API client module

**Files to create:** `src/ai/mod.rs`, `src/ai/gemini.rs`
**Files to modify:** `src/main.rs` (add `mod ai;`)
**Depends on:** none
**Parallel group:** P1

**Specification:**

Create a thin HTTP wrapper for the Gemini REST API.

**`src/ai/mod.rs`:**
```rust
pub mod gemini;
```

**`src/ai/gemini.rs`:**

The Gemini API endpoint:
```
POST https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:generateContent?key={API_KEY}
```

The API key is available at compile time via `env!("GEMINI_API_KEY")` (already in `.env`, loaded by `build.rs`).

Implement a struct `GeminiClient` with a `reqwest::Client` and one public method:

```rust
#[derive(Clone)]
pub struct GeminiClient {
    http: reqwest::Client,
    api_key: &'static str,
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
        // 1. Download all images from S3 GET URLs using self.http
        // 2. Base64-encode each image
        // 3. Build Gemini request with inline_data parts
        // 4. POST to Gemini API
        // 5. Parse JSON response → Vec<StudentScore>
    }
}

pub struct StudentScore {
    pub adm: i32,
    pub score: f64,
}
```

**Gemini request body structure (serde_json):**
```json
{
  "system_instruction": {
    "parts": [{"text": "You are an expert exam marker for Kenyan secondary school exams. Mark objectively and fairly. Award partial marks where the rubric allows."}]
  },
  "contents": [{
    "parts": [
      {"text": "Here is the marking scheme with questions, expected answers, rubric and correct answer examples:"},
      {"inline_data": {"mime_type": "image/jpeg", "data": "<base64>"}},
      {"text": "Now mark the following students' answer sheets."},
      {"text": "Student ADM 1234:"},
      {"inline_data": {"mime_type": "image/jpeg", "data": "<base64>"}},
      {"text": "Student ADM 5678:"},
      {"inline_data": {"mime_type": "image/jpeg", "data": "<base64>"}},
      {"text": "Total marks for this paper: 100\n\nReturn ONLY valid JSON:\n{\"results\": [{\"adm\": 1234, \"score\": 67.5}, {\"adm\": 5678, \"score\": 82.0}]}"}
    ]
  }],
  "generationConfig": {
    "responseMimeType": "application/json"
  }
}
```

**Important notes:**
- Gemini cannot fetch presigned S3 URLs directly as `fileUri`. Download images server-side with `reqwest` and send as base64 `inline_data`.
- Keep images in memory (do not write to disk).
- Use `base64::engine::general_purpose::STANDARD` for encoding (add `base64` crate to Cargo.toml if not already present — it is already a dependency).
- Parse Gemini's response: the `candidates[0].content.parts[0].text` field contains the JSON string. Deserialize into `Vec<StudentScore>`.
- On parse failure, log the raw Gemini response for debugging and return descriptive error.

**Add `mod ai;`** to `src/main.rs` alongside the existing `mod config; mod db; mod proto; mod services; mod types;`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] git commit: `feat: add Gemini API client for AI exam marking`

---

### Task S4: Create AiMarking proto trait bridge + service implementation

**Files to create:** `src/proto/services/ai_marking.rs`, `src/services/ai_marking.rs`
**Files to modify:** `src/proto/services/mod.rs`, `src/services/mod.rs`
**Depends on:** S1, S2, S3
**Parallel group:** —

**Specification:**

**Step 1 — Proto trait bridge: `src/proto/services/ai_marking.rs`**

Follow the exact pattern from `src/proto/services/authentication.rs`. Use `tonic::include_proto!("ai_marking")` at the top.

```rust
tonic::include_proto!("ai_marking");

use crate::types::{error::Result, token::Token};
pub use ai_marking_server::AiMarkingServer;
use std::future::Future;
use tonic::{Request, Response, Status};

pub trait AiMarking: Sync + Send + 'static + Sized {
    type Config: Sync + Send + 'static;
    fn new(config: Self::Config) -> AiMarkingServer<Self>;

    fn request_upload_urls(
        &self,
        token: Token,
        request: UploadUrlsRequest,
    ) -> impl Future<Output = Result<UploadUrlsResponse>> + Send;

    fn mark_paper(
        &self,
        token: Token,
        request: MarkPaperRequest,
    ) -> impl Future<Output = Result<MarkPaperResponse>> + Send;
}

#[tonic::async_trait]
impl<T: AiMarking> ai_marking_server::AiMarking for T {
    async fn request_upload_urls(
        &self,
        request: Request<UploadUrlsRequest>,
    ) -> std::result::Result<Response<UploadUrlsResponse>, Status> {
        let token = Token::from_metadata(request.metadata())?;
        let inner = request.into_inner();
        let response = self.request_upload_urls(token, inner).await?;
        Ok(Response::new(response))
    }

    async fn mark_paper(
        &self,
        request: Request<MarkPaperRequest>,
    ) -> std::result::Result<Response<MarkPaperResponse>, Status> {
        let token = Token::from_metadata(request.metadata())?;
        let inner = request.into_inner();
        let response = self.mark_paper(token, inner).await?;
        Ok(Response::new(response))
    }
}
```

**Note:** Check how `Token::from_metadata` works in the existing auth proto bridge. It extracts the Bearer token from the `authorization` metadata key. If the pattern is different (e.g. Token is extracted differently), adapt accordingly. Read `src/proto/services/authentication.rs` lines 40-50 for the exact pattern and replicate it.

**Step 2 — Service implementation: `src/services/ai_marking.rs`**

```rust
use crate::ai::gemini::GeminiClient;
use crate::config::Config;
use crate::config::storage::sign;
use crate::proto::services::ai_marking::*;
use crate::types::{error::Result, token::Token};
use std::sync::Arc;

pub struct AiMarkingService<C> {
    config: Arc<C>,
    gemini: GeminiClient,
}

impl<C: Config + Send + Sync + 'static> AiMarking for AiMarkingService<C> {
    type Config = Arc<C>;

    fn new(config: Self::Config) -> AiMarkingServer<Self> {
        AiMarkingServer::new(Self {
            config,
            gemini: GeminiClient::new(),
        })
    }

    async fn request_upload_urls(&self, token: Token, req: UploadUrlsRequest) -> Result<UploadUrlsResponse> {
        // 1. Validate token (get user — ensures the request is authenticated)
        let _user = self.config.validate(&token)?;

        // 2. Generate PUT URLs for scheme images
        let paper_num = req.paper.unwrap_or(0);
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

        // 3. Generate PUT URLs for student answer sheets
        let student_urls: Vec<StudentSignedUrls> = req.students.iter()
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

        Ok(UploadUrlsResponse { scheme_urls, student_urls })
    }

    async fn mark_paper(&self, token: Token, req: MarkPaperRequest) -> Result<MarkPaperResponse> {
        // 1. Validate token
        let _user = self.config.validate(&token)?;

        // 2. Generate GET URLs for all S3 keys (scheme + students)
        let scheme_get_urls: Vec<String> = req.scheme_keys.iter()
            .map(|key| sign::url(key, sign::GET_TTL, false))
            .collect();

        let student_data: Vec<(i32, Vec<String>)> = req.students.iter()
            .map(|s| {
                let urls: Vec<String> = s.keys.iter()
                    .map(|key| sign::url(key, sign::GET_TTL, false))
                    .collect();
                (s.adm, urls)
            })
            .collect();

        let student_count = student_data.len();

        // 3. Spawn async marking task (return immediately to client)
        let gemini = self.gemini.clone();
        let total_marks = req.total_marks;
        let school = req.school.clone();
        let exam = req.exam.clone();
        let subject = req.subject;
        let paper = req.paper;

        tokio::spawn(async move {
            match gemini.mark_paper(&scheme_get_urls, &student_data, total_marks).await {
                Ok(scores) => {
                    // Write grades to DB using existing grade write logic.
                    // Look at handle_mark_grades() in src/db/database/tables/actions.rs
                    // for the exact pattern. It does:
                    //   INSERT OR REPLACE INTO grades (school, exam, student, subject, paper, score, total, created, updated)
                    //   VALUES (?, ?, ?, ?, ?, ?, ?, unixepoch('now'), unixepoch('now'))
                    //
                    // After writing grades, call append_log() for each grade so the
                    // changelog picks it up and watchChanges streams it to clients.
                    //
                    // Also call the notify mechanism (if one exists) to wake up watch_loop
                    // immediately instead of waiting for the 1-second poll.
                    for score in &scores {
                        if let Err(e) = write_ai_grade(
                            &school, &exam, score.adm, subject, paper, score.score, total_marks,
                        ) {
                            tracing::error!("Failed to write AI grade for student {}: {}", score.adm, e);
                        }
                    }
                    tracing::info!(
                        "AI marking complete: {}/{} students scored for school={} exam={}",
                        scores.len(), student_count, school, exam
                    );
                }
                Err(e) => {
                    tracing::error!("Gemini marking failed for school={} exam={}: {}", school, exam, e);
                }
            }
        });

        Ok(MarkPaperResponse {
            accepted: true,
            message: format!("Marking {} students...", student_count),
        })
    }
}

/// Write a single AI-generated grade to the database and append a changelog entry.
///
/// This reuses the same write path as handle_mark_grades() in actions.rs.
/// Find that function, extract its grade-write logic, and call it here.
/// The key SQL is:
///   INSERT OR REPLACE INTO grades (school, exam, student, subject, paper, score, total, created, updated)
///   VALUES (?, ?, ?, ?, ?, ?, ?, unixepoch('now'), unixepoch('now'))
///
/// After the INSERT, call append_log(user, TBL_GRADES, OP_UPDATE, columns)
/// so that watchChanges picks up the new grade.
fn write_ai_grade(
    school: &str,
    exam: &str,
    student: i32,
    subject: i32,
    paper: Option<i32>,
    score: f64,
    total: i32,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use crate::db::database::CONN as conn;
    // Implementation: use conn to run the grade upsert SQL.
    // Look at how handle_mark_grades works in actions.rs and replicate.
    todo!("Wire up grade write using the existing DB connection pattern")
}
```

**Important implementation notes for the executor:**

1. **`write_ai_grade` function**: The `todo!()` must be replaced. Read `src/db/database/tables/actions.rs` and find `handle_mark_grades`. Copy its grade-insert logic. The key is the SQL upsert + `append_log()` call. The `append_log` is what makes the changelog binary file record the change so `watch_loop` picks it up.

2. **`GeminiClient` must be `Clone`**: Ensure `#[derive(Clone)]` is on `GeminiClient` (in S3). `reqwest::Client` implements Clone.

3. **Token validation**: `self.config.validate(&token)` should match the pattern used in `src/services/authentication.rs`. Read that file to see how the Config trait's `validate` method works.

4. **Changelog notification**: After writing grades, if there's a `Notify` mechanism to wake the `watch_loop` (check `src/services/sync.rs` for `tokio::sync::Notify`), call it. Otherwise the 1-second poll in `watch_loop` will pick up the changes naturally.

**Step 3 — Update `src/proto/services/mod.rs`:**

Add this line:
```rust
pub mod ai_marking;
```

**Step 4 — Update `src/services/mod.rs`:**

Add this line:
```rust
pub mod ai_marking;
```

**Update after completion:**
- [ ] Verify `cargo build` succeeds
- [ ] Mark this task `[x]`
- [ ] git commit: `feat: implement AiMarking service with RequestUploadUrls and MarkPaper handlers`

---

### Task S5: Register AiMarking service in server.rs

**Files to modify:** `src/server.rs`
**Depends on:** S4
**Parallel group:** —

**Specification:**

Update `src/server.rs` to create and register the `AiMarkingService`.

Add imports at the top:
```rust
use crate::proto::services::ai_marking::AiMarking;
use crate::services::ai_marking::AiMarkingService;
```

In the `start()` function, create the service and add it to the server builder:
```rust
let ai_marking = AiMarkingService::new(config.clone());
Server::builder()
    .add_service(authenticator)
    .add_service(sync)
    .add_service(ai_marking)
    .serve(addr)
    .await?;
```

Follow the exact pattern used for `authenticator` and `sync` — the `new()` method on the trait returns the tonic `Server` wrapper directly.

**Verification:** Run `cargo build` and verify it compiles. Optionally `cargo run` to confirm the server starts on port 50051 with three services registered.

**Update after completion:**
- [ ] Verify server compiles and starts
- [ ] Mark this task `[x]`
- [ ] git commit: `feat: register AiMarking gRPC service in server builder`
