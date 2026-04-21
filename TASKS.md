# TASKS.md

## Paper Generation Bug Fixes

---

### Task S1: Fix `select_random_questions` greedy algorithm + add clear error variant

**Files to create/modify:**
- `src/db/database/tables/question_bank.rs`
- `src/types/error.rs`
- `src/services/question_bank.rs`

**Depends on:** None
**Parallel group:** P1

**Specification:**

---

**Background:**

The `generate_paper` service handler fails with `Error::NothingToUpdate` (gRPC
`failed_precondition("nothing to update")`) whenever the question bank does not
have a combination of questions that *exactly* fills a topic's mark allocation.

The root cause is in `select_random_questions` in
`src/db/database/tables/question_bank.rs` (starting at line 211).

The current greedy loop is:
```rust
for row in rows {
    if remaining <= 0 {
        break;
    }
    let m = row.marks as i32;
    if m <= remaining {       // ← only picks questions that fit EXACTLY
        remaining -= m;
        selected.push(row);
    }
}
```

This loop **skips** any question whose marks exceed the remaining target. As a
result it can stop far short of the target. Example:
- target = 20, available questions (random order): [12, 12, 9]
- Greedy picks 12 (remaining=8), 12>8 skip, 9>8 skip → total=12 ✗
- Correct result: 12+9=21 (slight overshoot is acceptable)

The `generate_paper` service then checks:
```rust
if selected_marks < alloc.marks {
    return Err(Error::NothingToUpdate);
}
```
…and fails because 12 < 20.

---

**Fix A — `src/db/database/tables/question_bank.rs`**

Replace the greedy loop inside `select_random_questions` (the body after the SQL
query) with a version that continues picking until the cumulative total reaches
or exceeds `target_marks`. The last question in the selection may cause a slight
overshoot — that is intentional and acceptable:

```rust
// Greedy fill: pick questions in random order until cumulative marks >= target.
// Allows the last question to overshoot the target by its own mark value so
// that we never fail to fill a target just because no single small question
// fits the remainder.
let mut selected = Vec::new();
let mut current_marks = 0i32;
for row in rows {
    if current_marks >= target_marks as i32 {
        break;
    }
    current_marks += row.marks as i32;
    selected.push(row);
}
Ok(selected)
```

This is a minimal, low-risk change — just remove the `if m <= remaining` guard
and the `remaining` variable, and break only when we've already met the target.

---

**Fix B — `src/types/error.rs`**

Add a new error variant for "not enough questions" that gives a clearer gRPC
message than `NothingToUpdate`. Find the enum declaration (around line 92 where
`NothingToUpdate` is defined) and add:

```rust
NotEnoughQuestions,
```

In the `impl From<Error> for Status` block (around line 185), add a mapping:

```rust
Error::NotEnoughQuestions => Status::failed_precondition(
    "not enough questions in the bank for this topic and mark allocation",
),
```

Place it near the `NothingToUpdate` mapping.

---

**Fix C — `src/services/question_bank.rs`**

In the `generate_paper` handler, after `select_random_questions` returns, update
the insufficient-marks check to use `NotEnoughQuestions` and to account for the
fact that the new algorithm can overshoot (so `selected_marks >= alloc.marks`
is correct, not `==`):

Find this block (around line 534):
```rust
let selected_marks: i32 = selected.iter().map(|q| q.marks as i32).sum();
if selected_marks < alloc.marks {
    tracing::warn!(
        "generate_paper: not enough questions for topic {}: need {} marks, found {}",
        alloc.topic_id,
        alloc.marks,
        selected_marks
    );
    return Err(Error::NothingToUpdate);
}
```

Replace with:
```rust
if selected.is_empty() {
    tracing::warn!(
        "generate_paper: no questions found for topic {} (need {} marks)",
        alloc.topic_id,
        alloc.marks,
    );
    return Err(Error::NotEnoughQuestions);
}
let selected_marks: i32 = selected.iter().map(|q| q.marks as i32).sum();
if selected_marks < alloc.marks {
    tracing::warn!(
        "generate_paper: not enough questions for topic {}: need {} marks, found {}",
        alloc.topic_id,
        alloc.marks,
        selected_marks
    );
    return Err(Error::NotEnoughQuestions);
}
```

The `selected.is_empty()` guard catches the "topic has zero questions" case
before the marks check, giving a cleaner diagnostic.

---

**Update after completion:**
- [x] Mark this task `[x]`
- [x] `git add -A && git commit -m "fix: fix question selection algorithm, add NotEnoughQuestions error"`

---

### Task S2: Fix `finalize_paper` crash when school/exam data is missing from ledger DB

**Files to create/modify:**
- `src/services/question_bank.rs`

**Depends on:** None
**Parallel group:** P1 (disjoint from S1 — different service method)

**Specification:**

**Background:**

`finalize_paper` (around line 699 in `src/services/question_bank.rs`) queries
`schools`, `exams`, and `subjects` tables using `.get_result(conn)?`. If any of
these rows are not yet synced to the ledger's local SQLite, Diesel returns
`DieselError::NotFound`, which falls through the `impl From<DieselError> for Error`
match (there is no explicit arm for `NotFound`) and becomes `Error::Internal`.

The client receives a gRPC `Internal` status and shows an opaque "internal server
error". This is a confusing failure that blocks PDF generation even when questions
are available.

**Fix:**

Use `.optional()?` to turn a `NotFound` result into `None`, then fall back to
safe default strings. Change these three lookup blocks in `finalize_paper`:

```rust
// BEFORE — crashes with Error::Internal if row not found:
let school_info: SchoolInfoRow =
    sql_query("SELECT name, motto FROM schools WHERE id = ?")
        .bind::<Text, _>(&req.school)
        .get_result(conn)?;

let exam_info: ExamNameRow = sql_query("SELECT name FROM exams WHERE id = ?")
    .bind::<Text, _>(&req.exam)
    .get_result(conn)?;

let subject_info: SubjectNameRow = sql_query("SELECT name FROM subjects WHERE id = ?")
    .bind::<Integer, _>(req.subject)
    .get_result(conn)?;
```

Replace with:

```rust
// AFTER — falls back to placeholder strings if row not yet synced:
let school_info: Option<SchoolInfoRow> =
    sql_query("SELECT name, motto FROM schools WHERE id = ?")
        .bind::<Text, _>(&req.school)
        .get_result(conn)
        .optional()?;
let school_name = school_info
    .as_ref()
    .map(|s| s.name.as_str())
    .unwrap_or("School");
let school_motto = school_info.as_ref().and_then(|s| s.motto.as_deref());

let exam_info: Option<ExamNameRow> =
    sql_query("SELECT name FROM exams WHERE id = ?")
        .bind::<Text, _>(&req.exam)
        .get_result(conn)
        .optional()?;
let exam_name = exam_info.as_ref().map(|e| e.name.as_str()).unwrap_or("Exam");

let subject_info: Option<SubjectNameRow> =
    sql_query("SELECT name FROM subjects WHERE id = ?")
        .bind::<Integer, _>(req.subject)
        .get_result(conn)
        .optional()?;
let subject_name = subject_info
    .as_ref()
    .map(|s| s.name.as_str())
    .unwrap_or("Subject");
```

Then update the `generate_paper_pdf` call to use these local `&str` variables
instead of `&school_info.name` etc.:

```rust
let pdf_bytes = crate::pdf::generate_paper_pdf(
    school_name,           // ← was &school_info.name
    school_motto,          // ← was school_info.motto.as_deref()
    exam_name,             // ← was &exam_info.name
    subject_name,          // ← was &subject_info.name
    req.paper.map(|p| p as i16),
    req.grade as i16,
    &questions_data,
)
```

The `optional()` method is available on `QueryResult<T>` via
`diesel::OptionalExtension` — it converts `Err(NotFound)` into `Ok(None)` and
passes other errors through. Add this import at the top of the file if not already
present:

```rust
use diesel::OptionalExtension;
```

---

**Update after completion:**
- [x] Mark this task `[x]`
- [x] `git add -A && git commit -m "fix: graceful fallback in finalize_paper when school/exam not yet synced"`

---

### Task S3: Add `GetPaperQuestions` RPC to restore wizard state on client re-entry

**Files to create/modify:**
- `protos/services/question_bank.proto`
- `src/proto/services/question_bank.rs`
- `src/services/question_bank.rs`
- `src/db/database/tables/question_bank.rs` (add `get_full_paper_questions`)
- `build.rs` (no change needed — question_bank.proto is already registered)

**Depends on:** None (can run independently)
**Parallel group:** P2

**Specification:**

**Background:**

When a user exits the paper generation wizard and comes back to the same paper,
the Flutter client has lost its in-memory `_generatedQuestions` list. The server
still has the questions in `paper_questions` table. We need a `GetPaperQuestions`
RPC so the client can restore wizard state without forcing a full regeneration.

---

**Step 1 — `protos/services/question_bank.proto`**

Add the new RPC to the `QuestionBank` service block:

```protobuf
rpc GetPaperQuestions(GetPaperQuestionsRequest) returns (GetPaperQuestionsResponse);
```

Add the request/response message types (place near the other paper assembly messages
after `GetPaperPdfResponse`):

```protobuf
// === GetPaperQuestions ===

// Returns the currently assembled question list for a paper, ordered by position.
// Returns an empty list if no paper has been generated yet for this identity.
message GetPaperQuestionsRequest {
  string school = 1;
  string exam = 2;
  int32 subject = 3;
  optional int32 paper = 4;
  int32 grade = 5;
  optional int32 stream = 6;
}

message GetPaperQuestionsResponse {
  repeated PaperQuestion questions = 1;
}
```

After editing the proto, regenerate the Rust bindings by running:
```sh
cargo build
```
in the ledger directory (tonic-build re-generates on build).

---

**Step 2 — `src/db/database/tables/question_bank.rs`**

Add a helper that returns full `PaperQuestion` proto objects (with nested
`Question` data and signed image URLs) for a paper. Add after the existing
`get_paper_questions` function:

```rust
/// Load all paper questions for a paper with full question data (rubric + images),
/// ordered by position. Returns proto-ready structs ready for the gRPC response.
pub fn get_full_paper_questions(
    conn: &mut Conn,
    school: &str,
    exam: &str,
    subject: i32,
    paper: Option<i16>,
    grade: i16,
    stream: Option<i16>,
) -> Result<Vec<(i16, QuestionRow, Vec<RubricCriterionRow>, Vec<QuestionImageRow>)>> {
    let pqs = get_paper_questions(conn, school, exam, subject, paper, grade, stream)?;
    let mut result = Vec::with_capacity(pqs.len());
    for pq in &pqs {
        let row = get_question(conn, pq.question)?;
        let rubric = get_rubric_criteria(conn, pq.question)?;
        let images = get_question_images(conn, pq.question)?;
        result.push((pq.position, row, rubric, images));
    }
    Ok(result)
}
```

---

**Step 3 — `src/proto/services/question_bank.rs`**

Add `get_paper_questions` to the `QuestionBank` trait definition (find the existing
trait — it has methods like `generate_paper`, `finalize_paper`, etc.):

```rust
fn get_paper_questions(
    &self,
    token: Token,
    req: GetPaperQuestionsRequest,
) -> impl Future<Output = Result<GetPaperQuestionsResponse>> + Send;
```

Add the tonic blanket impl arm (inside the existing
`#[tonic::async_trait] impl<T: QuestionBank> question_bank_server::QuestionBank for T`
block). Follow the exact same pattern as the other RPCs in that file:

```rust
async fn get_paper_questions(
    &self,
    request: Request<GetPaperQuestionsRequest>,
) -> std::result::Result<Response<GetPaperQuestionsResponse>, Status> {
    let req = request.into_inner();
    // Token validation is handled by extract_token (or however other methods do it)
    let token = /* same token extraction as other methods in this file */;
    let response = self.get_paper_questions(token, req).await?;
    Ok(Response::new(response))
}
```

Follow **the exact same token extraction pattern** used by `generate_paper` and
`finalize_paper` in this file. Do not invent a new pattern.

---

**Step 4 — `src/services/question_bank.rs`**

Add the implementation to `impl<C: Send + Sync + 'static> QuestionBank for QuestionBankService<C>`:

```rust
// ── get_paper_questions ──────────────────────────────────────────────────

async fn get_paper_questions(
    &self,
    _token: Token,
    req: GetPaperQuestionsRequest,
) -> Result<GetPaperQuestionsResponse> {
    let questions = CONN.with(|cell| {
        let conn = &mut *cell.borrow_mut();

        let rows = question_bank::get_full_paper_questions(
            conn,
            &req.school,
            &req.exam,
            req.subject,
            req.paper.map(|p| p as i16),
            req.grade as i16,
            req.stream.map(|s| s as i16),
        )?;

        let paper_questions: Vec<PaperQuestion> = rows
            .iter()
            .map(|(pos, row, rubric, images)| PaperQuestion {
                position: *pos as i32,
                question: Some(build_question_proto(row, rubric, images, true)),
            })
            .collect();

        Ok::<_, Error>(paper_questions)
    })?;

    Ok(GetPaperQuestionsResponse { questions })
}
```

---

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] `git add -A && git commit -m "feat: add GetPaperQuestions RPC for wizard state restoration"`

