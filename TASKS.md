# TASKS.md
## Auto-Create Subject and Topic on Bulk Import

The `bulk_import_questions` endpoint currently does a strict lookup of both subject and topic. If either doesn't exist in the global catalog the whole import fails with `SubjectNotFound` / `TopicNotFound`. The import JSON file already carries all the data needed to create the subject and topic (`subject`, `curriculum`, `grade`, `topic`), so failing hard is wrong UX for a bulk-import tool.

---

### Task 05: Auto-create subject and topic during bulk import if they don't already exist
**Files to create/modify:** `src/services/question_bank.rs`
**Context files to read (if needed):** `AGENT.md`
**Depends on:** Tasks 01–02 (error-mapping fixes already applied)
**Parallel group:** P3

**Specification:**
In `src/services/question_bank.rs`, inside `bulk_import_questions`, replace the strict subject/topic lookup with an **upsert** (get-or-create) pattern. The import JSON already supplies everything needed.

#### Subject upsert
Replace:
```rust
let subject_row: SubjectIdRow =
    sql_query("SELECT id FROM subjects WHERE name = ? AND curriculum = ? LIMIT 1")
        ...
        .map_err(|_| { ... Error::SubjectNotFound })?;
```
With INSERT OR IGNORE + SELECT:
```sql
INSERT OR IGNORE INTO subjects (name, curriculum, created, updated)
VALUES (?, ?, strftime('%s','now'), strftime('%s','now'));

SELECT id FROM subjects WHERE name = ? AND curriculum = ? LIMIT 1;
```
The `INSERT OR IGNORE` is safe because `subjects` has a `UNIQUE(name, curriculum)` constraint — it is a no-op if the subject already exists. The SELECT immediately after always returns the row (whether pre-existing or just inserted).

#### Topic upsert
Replace:
```rust
let topic_row: TopicIdRow =
    sql_query("SELECT id FROM topics WHERE name = ? AND subject = ? AND grade = ? LIMIT 1")
        ...
        .map_err(|_| { ... Error::TopicNotFound })?;
```
With INSERT OR IGNORE + SELECT:
```sql
INSERT OR IGNORE INTO topics (subject, grade, name, created, updated)
VALUES (?, ?, ?, strftime('%s','now'), strftime('%s','now'));

SELECT id FROM topics WHERE name = ? AND subject = ? AND grade = ? LIMIT 1;
```
Same rationale: `UNIQUE(subject, grade, name)` constraint means `INSERT OR IGNORE` is a no-op if the topic already exists.

#### Both calls stay inside the same `conn.transaction(|conn| { ... })` block so the auto-created rows are visible to the subsequent question inserts in the same transaction.

#### Do not remove `SubjectNotFound` / `TopicNotFound` error variants from `error.rs` — they are still valid for other call sites. Just stop returning them from the bulk import path.

#### Error variants introduced in Task 01 (`SubjectNotFound`, `TopicNotFound`) are preserved for other uses.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

## Duplicate Question Detection

Questions with identical `(topic, text)` can currently be inserted multiple times. There is no uniqueness constraint at the schema level, no check in `insert_question`, and no guard in either `create_question` or `bulk_import_questions`. The fix spans four tasks: a migration, an error variant, a DB helper, and service wiring.

---

### Task 06: Migration — add UNIQUE(topic, text) index on questions table
**Files to create:**
- `ledger/migrations/2026-04-01-000000-0003_question_dedup/up.sql`
- `ledger/migrations/2026-04-01-000000-0003_question_dedup/down.sql`
**Depends on:** none
**Parallel group:** P1

**Specification:**

`up.sql` — first remove any duplicate rows that already exist in the database (keeping the lowest `id`), then create the unique index:

```sql
-- Remove duplicate questions, keeping the row with the lowest id
DELETE FROM questions
WHERE id NOT IN (
    SELECT MIN(id) FROM questions GROUP BY topic, text
);

-- Enforce uniqueness going forward
CREATE UNIQUE INDEX idx_questions_topic_text ON questions(topic, text);
```

`down.sql`:

```sql
DROP INDEX IF EXISTS idx_questions_topic_text;
```

Run `diesel migration run` from the `ledger/` directory after creating the files to apply the migration. Then run `diesel print-schema > src/db/schema/schema.rs` to regenerate the schema (the unique index doesn't add new columns, but the schema file should stay in sync).

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task 07: Add Error::QuestionAlreadyExists variant
**Files to modify:** `ledger/src/types/error.rs`
**Depends on:** none
**Parallel group:** P1

**Specification:**

In the `Error` enum (currently at ~line 24), add the new variant after `InvalidCurriculum`:

```rust
    #[error("question already exists")]
    QuestionAlreadyExists,
```

In `impl From<Error> for Status` add the mapping (place it near the other `already_exists` mappings, e.g. after `Error::RoleAlreadyExists`):

```rust
            Error::QuestionAlreadyExists => Status::already_exists("question already exists"),
```

Do not remove or rename any existing variants. Do not touch `Error::Conflict` — it remains the generic conflict used for DB-level unique violations before they are converted to domain-specific errors.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task 08: DB layer — add find_or_insert_question helper
**Files to modify:** `ledger/src/db/database/tables/question_bank.rs`
**Depends on:** none
**Parallel group:** P1

**Specification:**

Add a new public function `find_or_insert_question` directly after the existing `insert_question` function (currently ending at ~line 60). It performs a SELECT-first check (duplicate-safe inside a transaction) then inserts only if the question does not exist. Returns `(id, is_new)` where `is_new` is `false` when the question already existed.

```rust
/// Get-or-create a question by `(topic, text)`.
///
/// Returns `(id, is_new)`:
/// - `is_new = true`  — question was just inserted
/// - `is_new = false` — a question with the same (topic, text) already existed; the
///                      existing row's id is returned and nothing is written
///
/// Intended for use inside a transaction (e.g. bulk import) where
/// duplicate rows must be silently resolved rather than rejected.
pub fn find_or_insert_question(
    conn: &mut Conn,
    topic: i32,
    text: &str,
    marks: i16,
    example_answer: Option<&str>,
    created_by: &str,
) -> Result<(i32, bool)> {
    // Check for an existing question with the same (topic, text)
    let existing: Option<LastId> =
        sql_query("SELECT id FROM questions WHERE topic = ? AND text = ? LIMIT 1")
            .bind::<Integer, _>(topic)
            .bind::<Text, _>(text)
            .get_result(conn)
            .optional()?;

    if let Some(row) = existing {
        return Ok((row.id, false));
    }

    // No duplicate — insert the new question
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO questions \
         (topic, text, marks, example_answer, created, updated, created_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Integer, _>(topic)
    .bind::<Text, _>(text)
    .bind::<SmallInt, _>(marks)
    .bind::<Nullable<Text>, _>(example_answer)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(created_by)
    .execute(conn)?;

    let row: LastId = sql_query("SELECT last_insert_rowid() AS id").get_result(conn)?;
    Ok((row.id, true))
}
```

No other functions in the file need to change. `insert_question` stays as a plain INSERT — callers that want duplicate rejection (i.e. `create_question`) rely on the UNIQUE index raising `UniqueViolation` → `Error::Conflict`, which the service layer re-maps to `Error::QuestionAlreadyExists`.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task 09: Proto + service wiring — enforce dedup in create_question and bulk_import_questions
**Files to modify:**
- `ledger/protos/services/question_bank.proto`
- `ledger/src/services/question_bank.rs`
**Depends on:** Task 07, Task 08
**Parallel group:** P2 (sequential after P1)

**Specification:**

#### 1. Proto change — add `duplicates_skipped` field to `BulkImportResponse`

In `protos/services/question_bank.proto`, find `BulkImportResponse` (currently):

```protobuf
message BulkImportResponse {
  int32 questions_created = 1;
  repeated ImportError errors = 2;
  repeated int32 question_ids = 3;
}
```

Replace with:

```protobuf
message BulkImportResponse {
  int32 questions_created = 1;
  repeated ImportError errors = 2;
  repeated int32 question_ids = 3;
  int32 duplicates_skipped = 4;
}
```

#### 2. Service change — `create_question`

In `src/services/question_bank.rs`, the import line currently reads:

```rust
use crate::types::error::{Error, Result};
```

Change it to:

```rust
use crate::types::error::{Error, OnConflict, Result};
```

Then inside `create_question`, the `insert_question` call currently reads:

```rust
                let qid = question_bank::insert_question(
                    conn,
                    req.topic_id,
                    &req.text,
                    req.marks as i16,
                    req.example_answer.as_deref(),
                    &user_id,
                )?;
```

Replace with (apply `.on_conflict` before `?` to convert the generic DB conflict into the domain-specific error):

```rust
                let qid = question_bank::insert_question(
                    conn,
                    req.topic_id,
                    &req.text,
                    req.marks as i16,
                    req.example_answer.as_deref(),
                    &user_id,
                )
                .on_conflict(Error::QuestionAlreadyExists)?;
```

#### 3. Service change — `bulk_import_questions`

Inside the `for (idx, q) in parsed.questions.iter().enumerate()` loop in `bulk_import_questions`, the current `match question_bank::insert_question(...)` block:

```rust
                    match question_bank::insert_question(
                        conn,
                        topic_row.id,
                        &q.text,
                        q.marks as i16,
                        q.example_answer.as_deref(),
                        &user_id,
                    ) {
                        Ok(qid) => {
```

Replace `insert_question` with `find_or_insert_question` and handle the `is_new` boolean. Also add a `duplicates_skipped` counter before the loop and populate the new proto field in the response.

Add `let mut duplicates_skipped: i32 = 0;` alongside the other counters:

```rust
                let mut created_count: i32 = 0;
                let mut duplicates_skipped: i32 = 0;
                let mut errors: Vec<ImportError> = Vec::new();
                let mut question_ids: Vec<i32> = Vec::new();
```

Replace the `match question_bank::insert_question(...)` block with:

```rust
                    match question_bank::find_or_insert_question(
                        conn,
                        topic_row.id,
                        &q.text,
                        q.marks as i16,
                        q.example_answer.as_deref(),
                        &user_id,
                    ) {
                        Ok((qid, is_new)) => {
                            if !is_new {
                                // Question already exists in the catalog — record its id but
                                // do not count it as created and skip rubric re-insertion.
                                duplicates_skipped += 1;
                                question_ids.push(qid);
                                continue;
                            }
```

(The rest of the `Ok` arm — rubric insertion, `question_ids.push(qid)`, `created_count += 1` — remains unchanged.)

Update the `BulkImportResponse` construction to include `duplicates_skipped`:

```rust
                Ok::<_, Error>(BulkImportResponse {
                    questions_created: created_count,
                    duplicates_skipped,
                    errors,
                    question_ids,
                })
```

After editing, verify the code compiles with `cargo check` from the `ledger/` directory.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task
