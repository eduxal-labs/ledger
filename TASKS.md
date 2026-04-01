# TASKS.md — Question Bank & AI Marking Overhaul (Server)

> All tasks in this file are for the **ledger** (Rust server) codebase.
> Execute in order. Tasks within the same parallel group (P*) can run concurrently.
> Tasks with dependencies must wait for the referenced task to complete.

---

> **Commit rule:** Every executor agent MUST run `git add -A && git commit -m "<type>: <description>"` 
> immediately after completing its task. Do NOT defer commits. Types: `feat`, `fix`, `refactor`, `docs`, `chore`, `db`.

---

## Phase 0: Commit Uncommitted Changes

### Task S00: Commit any uncommitted changes [x]
**Files to modify:** None (git operation only)
**Depends on:** None
**Parallel group:** P0

**Specification:**
Before starting any work, check for uncommitted changes:
```bash
git status --short
```
If there are uncommitted changes, commit them:
```bash
git add -A && git commit -m "chore: commit pending changes before question bank overhaul"
```

**Expected outcome:** Clean working tree. All previous work preserved.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

## Phase 1: Proto Definitions

### Task S01: Create `protos/services/question_bank.proto` [x]
**Files to create:** `protos/services/question_bank.proto`
**Depends on:** None
**Parallel group:** P1

**Specification:**

Create a new proto file defining the QuestionBank gRPC service. Follow the same style as `protos/services/ai_marking.proto` (package name, import paths, naming conventions).

```proto
syntax = "proto3";
package question_bank;

service QuestionBank {
    // === System User Operations (question management) ===
    rpc CreateQuestion(CreateQuestionRequest) returns (CreateQuestionResponse);
    rpc UpdateQuestion(UpdateQuestionRequest) returns (UpdateQuestionResponse);
    rpc DeleteQuestion(DeleteQuestionRequest) returns (DeleteQuestionResponse);
    rpc BulkImportQuestions(BulkImportRequest) returns (BulkImportResponse);
    rpc RequestImageUploadUrls(ImageUploadUrlsRequest) returns (ImageUploadUrlsResponse);

    // === Teacher Operations (exam paper assembly) ===
    rpc GeneratePaper(GeneratePaperRequest) returns (GeneratePaperResponse);
    rpc RegenerateQuestion(RegenerateQuestionRequest) returns (RegenerateQuestionResponse);
    rpc EditPaperQuestion(EditPaperQuestionRequest) returns (EditPaperQuestionResponse);
    rpc FinalizePaper(FinalizePaperRequest) returns (FinalizePaperResponse);
    rpc GetPaperPdf(GetPaperPdfRequest) returns (GetPaperPdfResponse);

    // === Read Operations ===
    rpc ListQuestions(ListQuestionsRequest) returns (ListQuestionsResponse);
    rpc GetQuestion(GetQuestionRequest) returns (GetQuestionResponse);
    rpc GetQuestionGrades(GetQuestionGradesRequest) returns (GetQuestionGradesResponse);
    rpc GetMarkingStatus(MarkingStatusRequest) returns (MarkingStatusResponse);
}
```

**Messages to define:**

```
// === Shared types ===
message Question {
    int32 id = 1;
    int32 topic_id = 2;
    string text = 3;
    int32 marks = 4;
    optional string example_answer = 5;
    repeated RubricCriterion rubric = 6;
    repeated QuestionImage images = 7;
    int64 created = 8;
    int64 updated = 9;
}

message RubricCriterion {
    int32 position = 1;
    string criterion = 2;
    int32 marks = 3;
}

message QuestionImage {
    int32 id = 1;
    int32 position = 2;
    int32 context = 3;        // 0=question, 1=rubric, 2=example_answer
    string key = 4;           // R2 object key
    optional string url = 5;  // Presigned GET URL (populated in responses)
    optional string caption = 6;
}

// === CreateQuestion ===
message CreateQuestionRequest {
    int32 topic_id = 1;
    string text = 2;
    int32 marks = 3;
    optional string example_answer = 4;
    repeated RubricCriterionInput rubric = 5;
}
message RubricCriterionInput {
    string criterion = 1;
    int32 marks = 2;
}
message CreateQuestionResponse {
    Question question = 1;
}

// === UpdateQuestion ===
message UpdateQuestionRequest {
    int32 question_id = 1;
    optional string text = 2;
    optional int32 marks = 3;
    optional string example_answer = 4;
    repeated RubricCriterionInput rubric = 5; // if non-empty, replaces all criteria
}
message UpdateQuestionResponse {
    Question question = 1;
}

// === DeleteQuestion ===
message DeleteQuestionRequest {
    int32 question_id = 1;
}
message DeleteQuestionResponse {}

// === BulkImport ===
message BulkImportRequest {
    string json_content = 1;  // Full JSON string in topic.json format
}
message BulkImportResponse {
    int32 questions_created = 1;
    repeated ImportError errors = 2;
    repeated int32 question_ids = 3; // IDs of created questions (for subsequent image upload)
}
message ImportError {
    int32 index = 1;        // 0-based question index in the JSON array
    string message = 2;
}

// === Image Upload URLs ===
message ImageUploadUrlsRequest {
    repeated ImageUploadSpec images = 1;
}
message ImageUploadSpec {
    int32 question_id = 1;
    int32 position = 2;
    int32 context = 3;       // 0=question, 1=rubric, 2=example_answer
    optional string caption = 4;
}
message ImageUploadUrlsResponse {
    repeated ImageUploadUrl urls = 1;
}
message ImageUploadUrl {
    int32 question_id = 1;
    int32 position = 2;
    string key = 3;          // R2 object key: questions/{question_id}/{position}.webp
    string put_url = 4;      // Presigned PUT URL
}

// === GeneratePaper ===
message GeneratePaperRequest {
    string school = 1;
    string exam = 2;
    int32 subject = 3;
    optional int32 paper = 4;
    int32 grade = 5;
    optional int32 stream = 6;
    int32 total_marks = 7;
    repeated TopicAllocation topic_allocations = 8;
}
message TopicAllocation {
    int32 topic_id = 1;
    int32 marks = 2;
}
message GeneratePaperResponse {
    repeated PaperQuestion questions = 1;
}
message PaperQuestion {
    int32 position = 1;          // 0-based order on paper
    Question question = 2;       // Full question with rubric + images
}

// === RegenerateQuestion ===
message RegenerateQuestionRequest {
    string school = 1;
    string exam = 2;
    int32 subject = 3;
    optional int32 paper = 4;
    int32 grade = 5;
    optional int32 stream = 6;
    int32 position = 7;          // Position of question to replace
    int32 topic_id = 8;          // Topic constraint for replacement
    int32 marks = 9;             // Marks constraint for replacement
    repeated int32 exclude_ids = 10; // Question IDs already on the paper (avoid duplicates)
}
message RegenerateQuestionResponse {
    PaperQuestion replacement = 1;
}

// === EditPaperQuestion ===
message EditPaperQuestionRequest {
    int32 question_id = 1;
    optional string text = 2;
    optional int32 marks = 3;
    optional string example_answer = 4;
    repeated RubricCriterionInput rubric = 5; // if non-empty, replaces all criteria
}
message EditPaperQuestionResponse {
    Question question = 1;
}

// === FinalizePaper ===
message FinalizePaperRequest {
    string school = 1;
    string exam = 2;
    int32 subject = 3;
    optional int32 paper = 4;
    int32 grade = 5;
    optional int32 stream = 6;
}
message FinalizePaperResponse {
    string pdf_url = 1;          // Presigned GET URL for the PDF
    int64 pdf_expiry = 2;        // Unix timestamp when URL expires
}

// === GetPaperPdf ===
message GetPaperPdfRequest {
    string school = 1;
    string exam = 2;
    int32 subject = 3;
    optional int32 paper = 4;
    int32 grade = 5;
    optional int32 stream = 6;
}
message GetPaperPdfResponse {
    string pdf_url = 1;
    int64 pdf_expiry = 2;
}

// === ListQuestions ===
message ListQuestionsRequest {
    int32 topic_id = 1;
    optional int32 min_marks = 2;
    optional int32 max_marks = 3;
    int32 offset = 4;
    int32 limit = 5;
}
message ListQuestionsResponse {
    repeated Question questions = 1;
    int32 total = 2;
}

// === GetQuestion ===
message GetQuestionRequest {
    int32 question_id = 1;
}
message GetQuestionResponse {
    Question question = 1;
}

// === GetQuestionGrades ===
message GetQuestionGradesRequest {
    string school = 1;
    string exam = 2;
    int32 student = 3;
    int32 subject = 4;
    optional int32 paper = 5;
    int32 grade = 6;
    optional int32 stream = 7;
}
message GetQuestionGradesResponse {
    repeated QuestionGradeDetail grades = 1;
}
message QuestionGradeDetail {
    int32 question_id = 1;
    string question_text = 2;
    int32 question_marks = 3;
    float score = 4;
    optional string feedback = 5;
    repeated RubricCriterion rubric = 6;
}

// === MarkingStatus ===
enum MarkingPhase {
    QUEUED = 0;
    DOWNLOADING = 1;
    CACHING = 2;
    MARKING = 3;
    AGGREGATING = 4;
    COMPLETE = 5;
    FAILED = 6;
}
message MarkingStatusRequest {
    string school = 1;
    string exam = 2;
    int32 subject = 3;
    optional int32 paper = 4;
    int32 grade = 5;
    optional int32 stream = 6;
}
message MarkingStatusResponse {
    MarkingPhase phase = 1;
    string progress = 2;          // e.g. "5/30 students marked"
    optional string error = 3;
    optional int64 estimated_completion = 4; // unix timestamp
}
```

**Expected outcome:** Proto file compiles successfully with `tonic-prost-build`. All messages and service RPCs are defined.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

### Task S02: Update `build.rs` to compile `question_bank.proto` [x]
**Files to modify:** `build.rs`
**Depends on:** S01
**Parallel group:** P2

**Specification:**
Add `"./protos/services/question_bank.proto"` to the `compile_protos` input list in `build.rs`, alongside the existing `authentication.proto`, `sync.proto`, and `ai_marking.proto`.

After:
```rust
.compile_protos(
    &[
        "./protos/services/authentication.proto",
        "./protos/services/sync.proto",
        "./protos/services/ai_marking.proto",
        "./protos/services/question_bank.proto",  // ADD
    ],
    &["./protos/"],
)?;
```

Run `cargo build` to verify proto compilation succeeds.

**Expected outcome:** `cargo build` generates Rust bindings for the QuestionBank service in `target/`.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

### Task S03: Create proto binding module `src/proto/services/question_bank.rs` [x]
**Files to create:** `src/proto/services/question_bank.rs`
**Files to modify:** `src/proto/services/mod.rs`
**Depends on:** S02
**Parallel group:** P3

**Specification:**
Follow the exact pattern from `src/proto/services/ai_marking.rs`:

1. `tonic::include_proto!("question_bank");`
2. Import `crate::types::{error::Result, token::Token}`
3. `pub use question_bank_server::QuestionBankServer;`
4. Define `pub trait QuestionBank: Sync + Send + 'static + Sized` with:
   - `type Config: Sync + Send + 'static;`
   - `fn new(config: Self::Config) -> QuestionBankServer<Self>;`
   - One method per RPC, each taking `token: Token` + the request type, returning `impl Future<Output = Result<ResponseType>> + Send`
5. Implement `#[tonic::async_trait] impl<T: QuestionBank> question_bank_server::QuestionBank for T` with the standard token extraction pattern from each Request's metadata.
6. Add `pub mod question_bank;` to `src/proto/services/mod.rs`.

**Expected outcome:** `cargo build` succeeds. QuestionBank trait and server bindings compile.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

## Phase 2: Database Migration

### Task S04: Create Diesel migration for new tables [x]
**Files to create:** `migrations/2026-03-30-000000-0002_question_bank/up.sql`, `migrations/2026-03-30-000000-0002_question_bank/down.sql`
**Depends on:** None (can run in parallel with Phase 1)
**Parallel group:** P1

**Specification:**

`up.sql`:
```sql
-- Question bank tables (server-only — NOT synced to clients)

CREATE TABLE questions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    topic INTEGER NOT NULL,
    text TEXT NOT NULL,
    marks SMALLINT NOT NULL,
    example_answer TEXT,
    created BIGINT NOT NULL DEFAULT (unixepoch('now')),
    updated BIGINT NOT NULL DEFAULT (unixepoch('now')),
    created_by TEXT NOT NULL,
    FOREIGN KEY (topic) REFERENCES topics(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX idx_questions_topic ON questions(topic);
CREATE INDEX idx_questions_topic_marks ON questions(topic, marks);

CREATE TABLE rubric_criteria (
    question INTEGER NOT NULL,
    position SMALLINT NOT NULL,
    criterion TEXT NOT NULL,
    marks SMALLINT NOT NULL,
    PRIMARY KEY (question, position),
    FOREIGN KEY (question) REFERENCES questions(id) ON DELETE CASCADE
);

CREATE TABLE question_images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    question INTEGER NOT NULL,
    position SMALLINT NOT NULL,
    context SMALLINT NOT NULL,   -- 0=question, 1=rubric, 2=example_answer
    key TEXT NOT NULL,
    caption TEXT,
    FOREIGN KEY (question) REFERENCES questions(id) ON DELETE CASCADE
);
CREATE INDEX idx_question_images_question ON question_images(question);

CREATE TABLE question_grades (
    school TEXT NOT NULL,
    exam TEXT NOT NULL,
    student INTEGER NOT NULL,
    question INTEGER NOT NULL,
    score REAL NOT NULL,
    feedback TEXT,
    created BIGINT NOT NULL DEFAULT (unixepoch('now')),
    updated BIGINT NOT NULL DEFAULT (unixepoch('now')),
    PRIMARY KEY (school, exam, student, question),
    FOREIGN KEY (school) REFERENCES schools(id) ON DELETE CASCADE,
    FOREIGN KEY (exam) REFERENCES exams(id) ON DELETE CASCADE,
    FOREIGN KEY (school, student) REFERENCES students(school, adm) ON DELETE CASCADE,
    FOREIGN KEY (question) REFERENCES questions(id) ON DELETE CASCADE
);
CREATE INDEX idx_question_grades_student ON question_grades(school, student);
CREATE INDEX idx_question_grades_exam ON question_grades(school, exam);

CREATE TABLE paper_questions (
    school TEXT NOT NULL,
    exam TEXT NOT NULL,
    subject INTEGER NOT NULL,
    paper SMALLINT,
    grade SMALLINT NOT NULL,
    stream SMALLINT,
    question INTEGER NOT NULL,
    position SMALLINT NOT NULL,
    PRIMARY KEY (school, exam, subject, paper, grade, stream, question),
    FOREIGN KEY (question) REFERENCES questions(id) ON DELETE CASCADE
);

CREATE TABLE marking_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    school TEXT NOT NULL,
    exam TEXT NOT NULL,
    subject INTEGER NOT NULL,
    paper SMALLINT,
    grade SMALLINT NOT NULL,
    stream SMALLINT,
    phase SMALLINT NOT NULL DEFAULT 0,        -- MarkingPhase enum (0-6)
    progress TEXT NOT NULL DEFAULT '',         -- e.g. "5/30 students marked"
    error TEXT,
    total_students INTEGER NOT NULL DEFAULT 0,
    marked_students INTEGER NOT NULL DEFAULT 0,
    created BIGINT NOT NULL DEFAULT (unixepoch('now')),
    updated BIGINT NOT NULL DEFAULT (unixepoch('now')),
    FOREIGN KEY (school) REFERENCES schools(id) ON DELETE CASCADE,
    FOREIGN KEY (exam) REFERENCES exams(id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX idx_marking_queue_paper ON marking_queue(school, exam, subject, paper, grade, stream);
```

`down.sql`:
```sql
DROP TABLE IF EXISTS marking_queue;
DROP TABLE IF EXISTS paper_questions;
DROP TABLE IF EXISTS question_grades;
DROP TABLE IF EXISTS question_images;
DROP TABLE IF EXISTS rubric_criteria;
DROP TABLE IF EXISTS questions;
```

Run `diesel migration run` to verify.

**IMPORTANT:** Do NOT add any of these tables to `LogTable` enum in `src/services/sync.rs`. Do NOT add them to `SNAPSHOT_TABLE_ORDER`. These are server-only.

**Expected outcome:** Migration runs successfully. Tables exist in database.db.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

## Phase 3: Row Structs & CRUD

### Task S05: Add QueryableByName row structs for new tables [x]
**Files to modify:** `src/db/database/tables/rows.rs`
**Depends on:** S04
**Parallel group:** P4

**Specification:**
Add row structs following the exact pattern in rows.rs (e.g. `UserRow`, `SchoolRow`):

```rust
// ---------------------------------------------------------------------------
// Question Bank — Server-Only Tables
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct QuestionRow {
    #[diesel(sql_type = Integer)]
    pub id: i32,
    #[diesel(sql_type = Integer)]
    pub topic: i32,
    #[diesel(sql_type = Text)]
    pub text: String,
    #[diesel(sql_type = SmallInt)]
    pub marks: i16,
    #[diesel(sql_type = Nullable<Text>)]
    pub example_answer: Option<String>,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
    #[diesel(sql_type = Text)]
    pub created_by: String,
}

#[derive(QueryableByName)]
pub struct RubricCriterionRow {
    #[diesel(sql_type = Integer)]
    pub question: i32,
    #[diesel(sql_type = SmallInt)]
    pub position: i16,
    #[diesel(sql_type = Text)]
    pub criterion: String,
    #[diesel(sql_type = SmallInt)]
    pub marks: i16,
}

#[derive(QueryableByName)]
pub struct QuestionImageRow {
    #[diesel(sql_type = Integer)]
    pub id: i32,
    #[diesel(sql_type = Integer)]
    pub question: i32,
    #[diesel(sql_type = SmallInt)]
    pub position: i16,
    #[diesel(sql_type = SmallInt)]
    pub context: i16,
    #[diesel(sql_type = Text)]
    pub key: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub caption: Option<String>,
}

#[derive(QueryableByName)]
pub struct QuestionGradeRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Text)]
    pub exam: String,
    #[diesel(sql_type = Integer)]
    pub student: i32,
    #[diesel(sql_type = Integer)]
    pub question: i32,
    #[diesel(sql_type = Float)]
    pub score: f32,
    #[diesel(sql_type = Nullable<Text>)]
    pub feedback: Option<String>,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

#[derive(QueryableByName)]
pub struct PaperQuestionRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Text)]
    pub exam: String,
    #[diesel(sql_type = Integer)]
    pub subject: i32,
    #[diesel(sql_type = Nullable<SmallInt>)]
    pub paper: Option<i16>,
    #[diesel(sql_type = SmallInt)]
    pub grade: i16,
    #[diesel(sql_type = Nullable<SmallInt>)]
    pub stream: Option<i16>,
    #[diesel(sql_type = Integer)]
    pub question: i32,
    #[diesel(sql_type = SmallInt)]
    pub position: i16,
}

#[derive(QueryableByName)]
pub struct MarkingQueueRow {
    #[diesel(sql_type = Integer)]
    pub id: i32,
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Text)]
    pub exam: String,
    #[diesel(sql_type = Integer)]
    pub subject: i32,
    #[diesel(sql_type = Nullable<SmallInt>)]
    pub paper: Option<i16>,
    #[diesel(sql_type = SmallInt)]
    pub grade: i16,
    #[diesel(sql_type = Nullable<SmallInt>)]
    pub stream: Option<i16>,
    #[diesel(sql_type = SmallInt)]
    pub phase: i16,
    #[diesel(sql_type = Text)]
    pub progress: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub error: Option<String>,
    #[diesel(sql_type = Integer)]
    pub total_students: i32,
    #[diesel(sql_type = Integer)]
    pub marked_students: i32,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}
```

These do NOT need `row_key()`, `school_id()`, or `From<&Row> for Insert` implementations since they are not synced tables.

**Expected outcome:** `cargo build` succeeds with new row structs.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

### Task S06: Add CRUD functions for question bank tables [x]
**Files to create:** `src/db/database/tables/question_bank.rs`
**Files to modify:** `src/db/database/tables/mod.rs`
**Depends on:** S05
**Parallel group:** P5

**Specification:**
Create a new module `question_bank.rs` (unlike synced tables which go in `insert.rs`/`update.rs`/`delete.rs`, these server-only tables get their own module for cleaner separation).

Follow the raw SQL pattern from `insert.rs` — `sql_query().bind::<Type, _>().execute()`.

Functions to implement:

```rust
use crate::types::error::Result;
use diesel::SqliteConnection as Conn;

// === Questions ===
pub fn insert_question(conn: &mut Conn, topic: i32, text: &str, marks: i16,
    example_answer: Option<&str>, created_by: &str) -> Result<i32>;
    // Returns the new question ID (use RETURNING or last_insert_rowid)

pub fn update_question(conn: &mut Conn, id: i32, text: Option<&str>,
    marks: Option<i16>, example_answer: Option<Option<&str>>) -> Result<()>;
    // Dynamic SET clause for non-None fields. Update `updated` timestamp.

pub fn delete_question(conn: &mut Conn, id: i32) -> Result<()>;
    // CASCADE handles rubric_criteria, question_images, question_grades, paper_questions

pub fn get_question(conn: &mut Conn, id: i32) -> Result<QuestionRow>;

pub fn list_questions(conn: &mut Conn, topic: i32, min_marks: Option<i16>,
    max_marks: Option<i16>, offset: i32, limit: i32) -> Result<(Vec<QuestionRow>, i32)>;
    // Returns (rows, total_count). Build WHERE dynamically.

pub fn count_questions_by_topic(conn: &mut Conn, topic: i32) -> Result<i32>;

// Fetch random questions for paper generation
pub fn select_random_questions(conn: &mut Conn, topic: i32, target_marks: i16,
    exclude_ids: &[i32]) -> Result<Vec<QuestionRow>>;
    // ORDER BY RANDOM(), greedy-fill to target_marks

// === Rubric Criteria ===
pub fn insert_rubric_criteria(conn: &mut Conn, question: i32,
    criteria: &[(i16, &str, i16)]) -> Result<()>;
    // (position, criterion, marks) — bulk insert

pub fn replace_rubric_criteria(conn: &mut Conn, question: i32,
    criteria: &[(i16, &str, i16)]) -> Result<()>;
    // DELETE all existing + INSERT new (within caller's transaction)

pub fn get_rubric_criteria(conn: &mut Conn, question: i32) -> Result<Vec<RubricCriterionRow>>;

// === Question Images ===
pub fn insert_question_image(conn: &mut Conn, question: i32, position: i16,
    context: i16, key: &str, caption: Option<&str>) -> Result<i32>;

pub fn get_question_images(conn: &mut Conn, question: i32) -> Result<Vec<QuestionImageRow>>;

pub fn delete_question_images(conn: &mut Conn, question: i32) -> Result<()>;

// === Question Grades ===
pub fn upsert_question_grade(conn: &mut Conn, school: &str, exam: &str,
    student: i32, question: i32, score: f32, feedback: Option<&str>) -> Result<()>;
    // INSERT ... ON CONFLICT DO UPDATE

pub fn get_question_grades_for_student(conn: &mut Conn, school: &str, exam: &str,
    student: i32, question_ids: &[i32]) -> Result<Vec<QuestionGradeRow>>;
    // Fetch per-question grades for a specific student on a paper

// === Paper Questions ===
pub fn insert_paper_questions(conn: &mut Conn, school: &str, exam: &str,
    subject: i32, paper: Option<i16>, grade: i16, stream: Option<i16>,
    questions: &[(i32, i16)]) -> Result<()>;
    // (question_id, position) — bulk insert

pub fn get_paper_questions(conn: &mut Conn, school: &str, exam: &str,
    subject: i32, paper: Option<i16>, grade: i16, stream: Option<i16>)
    -> Result<Vec<PaperQuestionRow>>;

pub fn delete_paper_questions(conn: &mut Conn, school: &str, exam: &str,
    subject: i32, paper: Option<i16>, grade: i16, stream: Option<i16>) -> Result<()>;

pub fn replace_paper_question_at_position(conn: &mut Conn, school: &str, exam: &str,
    subject: i32, paper: Option<i16>, grade: i16, stream: Option<i16>,
    position: i16, new_question_id: i32) -> Result<()>;

// === Marking Queue ===
pub fn upsert_marking_queue(conn: &mut Conn, school: &str, exam: &str,
    subject: i32, paper: Option<i16>, grade: i16, stream: Option<i16>,
    phase: i16, total_students: i32) -> Result<i32>;
    // Returns queue row ID

pub fn update_marking_status(conn: &mut Conn, id: i32, phase: i16,
    progress: &str, marked_students: i32, error: Option<&str>) -> Result<()>;

pub fn get_marking_status(conn: &mut Conn, school: &str, exam: &str,
    subject: i32, paper: Option<i16>, grade: i16, stream: Option<i16>)
    -> Result<Option<MarkingQueueRow>>;
```

Add `pub mod question_bank;` to `src/db/database/tables/mod.rs`.

**Expected outcome:** `cargo build` succeeds. All CRUD functions compile.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

## Phase 4: QuestionBank gRPC Service

### Task S07: Create `src/services/question_bank.rs` — question CRUD RPCs [x]
**Files to create:** `src/services/question_bank.rs`
**Files to modify:** `src/services/mod.rs`
**Depends on:** S03, S06
**Parallel group:** P6

**Specification:**
Create the QuestionBank service implementation. Follow `src/services/ai_marking.rs` as the template:

1. `pub struct QuestionBankService<C> { config: Arc<C> }` — no queue needed for CRUD RPCs.
2. `impl<C: Send + Sync + 'static> QuestionBank for QuestionBankService<C>` — implement the trait from S03.

Implement these RPCs:

**`create_question`:**
- Validate: token user level >= System (level 1 or 2)
- Validate: `text` non-empty, `marks > 0`, `topic_id` exists in `topics` table
- Validate: sum of rubric marks == question marks
- Run in `CONN.with()` transaction:
  - `insert_question` → get new ID
  - `insert_rubric_criteria` with position = index
- Return full `Question` with rubric criteria

**`update_question`:**
- Validate: token user level >= System
- Run in transaction:
  - `update_question` with non-None fields
  - If `rubric` is non-empty: `replace_rubric_criteria`
- Return updated `Question`

**`delete_question`:**
- Validate: token user level >= System
- `delete_question` — CASCADE handles children

**`list_questions`:**
- Validate: token user level >= System
- Call `list_questions` with filters
- For each question, also load rubric + images
- Return `ListQuestionsResponse`

**`get_question`:**
- Validate: token user level >= System
- Load question + rubric + images
- Sign image keys → presigned GET URLs

**`bulk_import_questions`:**
- Validate: token user level >= System
- Parse `json_content` as the topic.json format:
  ```json
  { "subject": "...", "curriculum": "...", "grade": N, "topic": "...", "questions": [...] }
  ```
- Look up subject by `(name, curriculum)` — curriculum "844" → 1, "cbc" → 0
- Look up topic by `(name, subject.id, grade)`
- For each question:
  - Validate marks = sum of rubric marks
  - Insert question + rubric criteria
  - Track created IDs
- Return `BulkImportResponse` with counts, errors, and question IDs

**`request_image_upload_urls`:**
- Validate: token user level >= System
- For each `ImageUploadSpec`:
  - Generate key: `questions/{question_id}/{position}.webp`
  - Sign a PUT URL via `config::storage::sign::url()`
  - Insert/update `question_images` row with the key
- Return presigned PUT URLs

Add `pub mod question_bank;` to `src/services/mod.rs`.

**Expected outcome:** Question CRUD RPCs compile and are logically correct.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

### Task S08: Implement paper generation RPCs (`generate_paper`, `regenerate_question`, `edit_paper_question`) [x]
**Files to modify:** `src/services/question_bank.rs`
**Depends on:** S07
**Parallel group:** P7

**Specification:**

**`generate_paper`:**
- Validate: token is authenticated
- Validate: exam exists, paper exists (if specified), all topic_ids exist
- Validate: sum of `topic_allocations[*].marks == total_marks`
- For each topic allocation:
  - Call `select_random_questions(topic_id, marks, exclude_ids=[])`
  - Greedy algorithm: pick questions (preferring exact fits) until marks target reached
  - If not enough questions in a topic, return an error listing which topics are short
- Assign positions 0..N across all selected questions
- Write to `paper_questions` table
- Return `GeneratePaperResponse` with full question data (text, rubric, images with signed URLs)

**`regenerate_question`:**
- Validate: token is authenticated
- Load current paper_questions for this paper
- Find the question at the given position
- Call `select_random_questions(topic_id, marks, exclude_ids=current_paper_question_ids)`
- Replace the paper_question at that position
- Return the replacement `PaperQuestion`

**`edit_paper_question`:**
- Validate: token is authenticated
- This actually edits the `questions` row itself (persists to DB, improving the question bank)
- Delegates to `update_question` + `replace_rubric_criteria` if rubric provided
- Return updated `Question`

**Expected outcome:** Paper generation RPCs compile and implement correct selection logic.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

### Task S09: Implement `finalize_paper` and `get_paper_pdf` RPCs + PDF generation [x]
**Files to create:** `src/pdf.rs` (PDF generation module)
**Files to modify:** `src/services/question_bank.rs`, `Cargo.toml`
**Depends on:** S08
**Parallel group:** P8

**Specification:**

**Research:** Evaluate `genpdf` (https://crates.io/crates/genpdf) for PDF generation. It wraps `printpdf` with a higher-level API. Add to Cargo.toml:
```toml
genpdf = "0.3"
```
If `genpdf` is insufficient (e.g., no image embedding), fall back to `printpdf` directly.

**`src/pdf.rs`:**
Create a module that generates an exam paper PDF:

```rust
pub fn generate_paper_pdf(
    school_name: &str,
    school_motto: Option<&str>,
    exam_name: &str,
    subject_name: &str,
    paper_number: Option<i16>,
    grade: i16,
    date: &str,           // formatted date string
    questions: &[(QuestionRow, Vec<RubricCriterionRow>, Vec<(QuestionImageRow, Vec<u8>)>)],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
```

Layout:
1. **Header:** School name (bold, centered), motto (italic, centered if present)
2. **Sub-header:** Exam name | Subject | Paper N | Grade/Form | Date
3. **Instructions:** "Answer ALL questions in the spaces provided" (or configurable)
4. **Questions:** Numbered sequentially starting at 1
   - Question text with "(X marks)" appended
   - Images embedded at correct positions (download from R2, embed in PDF)
   - Captions below images in italics
   - Sub-parts indented with (a), (b), (c)
   - NO answer spaces — students write on blank paper
5. **Footer:** Page numbers "Page X of Y"

**`finalize_paper`:**
- Load paper_questions for this paper (ordered by position)
- For each question: load text, rubric, images
- Download question images from R2 (presigned GET URLs)
- Load school name + motto from `schools` table
- Load exam name from `exams` table
- Load subject name from `subjects` table
- Call `generate_paper_pdf(...)` → get `Vec<u8>`
- Upload PDF to R2: `schools/{school}/exams/{exam}/papers/{subject}_{paper}/paper.pdf`
- Return presigned GET URL + expiry

**`get_paper_pdf`:**
- Check if PDF key exists: `schools/{school}/exams/{exam}/papers/{subject}_{paper}/paper.pdf`
- Return presigned GET URL + expiry
- If not found, return appropriate error

**Expected outcome:** PDF generation produces valid PDF files. FinalizePaper uploads to R2 and returns download URL.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

## Phase 5: AI Marking Overhaul

### Task S10: Remove Anthropic client [x]
**Files to delete:** `src/ai/anthropic.rs`
**Files to modify:** `src/ai/mod.rs`, `Cargo.toml`
**Depends on:** None
**Parallel group:** P1

**Specification:**
1. Delete `src/ai/anthropic.rs` entirely.
2. In `src/ai/mod.rs`: remove `pub mod anthropic;`
3. In `Cargo.toml`: no Anthropic-specific dependency exists (they share `reqwest`), so no Cargo changes needed.
4. Check `src/services/ai_marking.rs` for any `anthropic` imports — there should be none (it only uses `GeminiClient`), but verify.
5. Remove `ANTHROPIC_API_KEY` from `.env` if present (but don't break the build — it's not referenced via `env!()` in any remaining file after anthropic.rs is deleted).

**Expected outcome:** `cargo build` succeeds with no Anthropic references.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

### Task S11: Select production Gemini model and update `src/ai/gemini.rs` [x]
**Files to modify:** `src/ai/gemini.rs`
**Depends on:** S10
**Parallel group:** P9

**Specification:**

The current model `gemini-3.1-flash-lite-preview` returns 503 errors because it's in preview. Research the current production-ready Gemini models at https://ai.google.dev/gemini-api/docs/models/gemini and select:
- A stable, production-ready model (NOT preview)
- Strong accuracy for structured text extraction from handwritten images
- Cost-effective for high volume
- Supports context caching
- Supports JSON mode / structured output

Recommended candidates to evaluate:
- `gemini-2.0-flash` — latest production Flash model, good balance of speed/accuracy
- `gemini-2.5-flash` — already the fallback model, production-ready
- `gemini-1.5-flash` — older but stable

Update `gemini.rs`:
1. Change `const MODEL` to the selected production model
2. Change `const FALLBACK_MODEL` to the next best option
3. Keep the 503 → fallback retry logic but verify it works with the new model names
4. Test by running `cargo build` and manually checking model availability via the API

**Expected outcome:** Production model selected. No more 503 errors in normal operation.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

### Task S12: Rewrite marking for per-question marking architecture [x]
**Files to modify:** `src/services/ai_marking.rs`, `src/ai/gemini.rs`
**Depends on:** S06, S11
**Parallel group:** P10

**Specification:**

**Overview:** The current system sends ONE Gemini request per student containing ALL questions + the full marking scheme image. The new system sends ONE Gemini request PER QUESTION PER STUDENT.

**Changes to `src/ai/gemini.rs`:**

Add a new method for per-question marking:
```rust
pub async fn mark_single_question(
    &self,
    system_cache_name: &str,    // System prompt cache (shared across all)
    student_images_b64: &[String], // Student's answer sheet images
    question_text: &str,
    question_marks: i16,
    rubric_criteria: &[(String, i16)], // (criterion, marks)
    question_images_b64: &[(String, Option<String>)], // (image_b64, caption)
) -> Result<QuestionScore, Box<dyn Error + Send + Sync>>;
```

Where:
```rust
pub struct QuestionScore {
    pub score: f64,
    pub feedback: String,
}
```

The prompt for each question:
```
You are marking ONE specific question on a student's answer sheet.

QUESTION: {question_text}
TOTAL MARKS: {question_marks}

RUBRIC CRITERIA:
1. {criterion} ({marks} marks)
2. ...

[Question images if any]

The student's answer sheets are shown above. Find the answer to THIS specific question and mark it according to the rubric criteria.

Return ONLY valid JSON:
{"score": <number>, "feedback": "<one paragraph justification>"}
```

**Changes to `src/services/ai_marking.rs`:**

Replace the `mark_and_write` function with a new flow:

1. **Status tracking:** Before starting, UPSERT a `marking_queue` row with phase=QUEUED.

2. **Download phase:** Download all student answer sheet images (same as now). Update marking_queue phase=DOWNLOADING.

3. **Cache phase:** Create TWO types of caches:
   - System prompt cache: shared across ALL requests (TTL 5 min). Contains the KCSE marking instructions (the existing ~4000 word SYSTEM_INSTRUCTION). Reuse if already cached.
   - Per-student answer sheet cache: For each student, create a cache with their answer sheet images. TTL = duration of that student's marking.
   - Update marking_queue phase=CACHING.

4. **Marking phase (per-question):** Update marking_queue phase=MARKING.
   - Load paper_questions for this paper → get ordered list of question IDs
   - For each question: load text, rubric_criteria, question_images from DB
   - Dynamic concurrency: Track current RPM. For N students × M questions:
     - Calculate max concurrent requests based on Gemini API quota (default: 60 RPM for flash models)
     - Use `tokio::sync::Semaphore` with dynamic permits
   - For each student, for each question:
     - `mark_single_question(system_cache, student_cache, question, rubric, images)`
     - On success: `upsert_question_grade(school, exam, student, question_id, score, feedback)`
     - Update progress: increment marked_students in marking_queue
   - Retry failed individual questions (3 attempts with 2s/4s backoff)

5. **Aggregation phase:** Update marking_queue phase=AGGREGATING.
   - For each student:
     - Sum question_grades.score → paper total
     - Write/update `grades` table (same as current `write_grades_to_db` but using question_grades sum)
     - Write changelog entries for grades + aiusage
   - If exam type == Assessment (type=2):
     - Group question_grades by topic (via questions.topic)
     - For each topic: `topic_score / topic_total` → UPSERT mastery table
     - Write changelog entries for mastery

6. **Completion:** Update marking_queue phase=COMPLETE (or FAILED with error message).

**Keep backward compatibility:** The existing `MarkPaper` RPC signature doesn't change. The flow is:
- Client calls `MarkPaper` → returns accepted=true
- Server now checks if paper has `paper_questions` entries
  - If yes → use new per-question marking flow
  - If no → fall back to old image-based marking (for papers not generated via question bank)

**Expected outcome:** Per-question marking works. Question-level grades stored. Aggregation produces correct paper totals and mastery scores. Status tracking via marking_queue table.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

### Task S13: Implement `get_marking_status` RPC [x]
**Files to modify:** `src/services/question_bank.rs` (or `src/services/ai_marking.rs` — pick one)
**Depends on:** S12
**Parallel group:** P11

**Specification:**
Implement the `GetMarkingStatus` RPC. Since the marking_queue table persists status:

1. Query `marking_queue` for the given (school, exam, subject, paper, grade, stream)
2. If not found → return QUEUED with empty progress (or NOT_FOUND error if no marking was ever requested)
3. If found → map row to `MarkingStatusResponse`:
   - phase → MarkingPhase enum value
   - progress → progress string
   - error → error message (if phase == FAILED)
   - estimated_completion → compute based on marked_students/total_students and elapsed time

**Note:** This RPC can live in either the QuestionBank service or the AiMarking service. Since `ai_marking.proto` already exists and this is marking-related, it may be cleaner to add the RPC to `ai_marking.proto` and implement it in `ai_marking.rs`. However, since we defined it in `question_bank.proto`, implement it in `question_bank.rs`.

**Expected outcome:** Client can poll marking status.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

## Phase 6: Wire Service into Server

### Task S14: Register QuestionBank service in `src/server.rs`
**Files to modify:** `src/server.rs`
**Depends on:** S07 (at minimum — ideally after S08, S09, S13)
**Parallel group:** P12

**Specification:**

Add the QuestionBank service to the tonic server, following the exact pattern of the existing 3 services:

```rust
use crate::proto::services::question_bank::QuestionBank;
use crate::services::question_bank::QuestionBankService;

// In start():
let question_bank = QuestionBankService::new(config.clone());

Server::builder()
    .add_service(authenticator)
    .add_service(sync)
    .add_service(ai_marking)
    .add_service(question_bank)  // ADD
    .serve(addr)
    .await?;
```

**Expected outcome:** Server starts with 4 gRPC services. QuestionBank RPCs are reachable.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

## Phase 7: Integration & Testing

### Task S15: Add `get_question_grades` RPC implementation [x]
**Files to modify:** `src/services/question_bank.rs`
**Depends on:** S07, S06
**Parallel group:** P7 (can parallel with S08)

**Specification:**

Implement `get_question_grades`:
1. Validate token is authenticated
2. Load paper_questions for the given (school, exam, subject, paper, grade, stream) → get question IDs
3. Load question_grades for the given student + those question IDs
4. For each question: also load the question text, marks, and rubric
5. Join them together → `GetQuestionGradesResponse` with `QuestionGradeDetail` entries

**Expected outcome:** Client can fetch per-question breakdown for a student's paper.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

### Task S16: Verify end-to-end: proto compilation, service registration, basic RPC flow
**Files to modify:** None (verification only)
**Depends on:** S14
**Parallel group:** P13

**Specification:**
1. Run `cargo build` — must succeed with zero errors
2. Run `cargo clippy` — fix any warnings
3. Start the server (`cargo run`) — verify it starts without panics
4. Verify the following log lines appear:
   - gRPC server starts on port 50051
   - No "missing module" or "unresolved import" errors
5. Test with `grpcurl` or manual client if available:
   - `ListQuestions` with a valid topic_id → should return empty list
   - `GetMarkingStatus` with a non-existent paper → should return appropriate response

**Expected outcome:** Server compiles, starts, and responds to QuestionBank RPCs.

**Commit:** `git add -A && git commit -m "<type>: <description of what this task did>"`

---

## Dependency Graph Summary

```
S01 (proto def)
 └→ S02 (build.rs)
     └→ S03 (proto bindings)
         └→ S07 (CRUD RPCs) ──→ S08 (paper gen) ──→ S09 (PDF + finalize)
                                                              │
S04 (migration) ──→ S05 (row structs) ──→ S06 (CRUD fns) ───┘
                                               │
S10 (rm anthropic) ──→ S11 (gemini model) ──→ S12 (per-Q marking) ──→ S13 (marking status)
                                                                              │
                                              S14 (wire server) ←────────────┘
                                               │
                                              S15 (question grades)
                                               │
                                              S16 (verification)
```

## Parallel Execution Groups

| Group | Tasks | Notes |
|-------|-------|-------|
| P1 | S01, S04, S10 | Proto definition, migration, and Anthropic removal are independent |
| P2 | S02 | Depends on S01 |
| P3 | S03 | Depends on S02 |
| P4 | S05 | Depends on S04 |
| P5 | S06 | Depends on S05 |
| P6 | S07 | Depends on S03 + S06 |
| P7 | S08, S15 | Both depend on S07 |
| P8 | S09 | Depends on S08 |
| P9 | S11 | Depends on S10 |
| P10 | S12 | Depends on S06 + S11 |
| P11 | S13 | Depends on S12 |
| P12 | S14 | Depends on all service tasks |
| P13 | S16 | Final verification |
