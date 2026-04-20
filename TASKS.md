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
