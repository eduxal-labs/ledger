# TASKS.md

## Feature: AI Usage Tracking & Gemini Model Update (Backend)

### Task B1: Update Gemini model version in backend

**Files to modify:** `src/ai/gemini.rs`
**Depends on:** none
**Parallel group:** P1

**Specification:**

The current URL in `src/ai/gemini.rs` is:
```
const URL: &str = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:generateContent";
```

**Confirmed by project owner:** Use **Gemini 3.1 Pro** (`gemini-3.1-pro-preview`).

Update the `URL` constant to:
```
const URL: &str = "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.1-pro-preview:generateContent";
```

> **Note:** Gemini 3.1 Pro is currently a Preview model. Preview models may have more restrictive
> rate limits and will be deprecated with at least 2 weeks notice. If Google changes the model
> string (e.g. adds a date suffix like `gemini-3.1-pro-preview-06-2026`), update this constant.
> Alternatively, use `gemini-pro-latest` to always point to the newest Pro release automatically.

**Update after completion:**
- [x] Confirm model choice with project owner — **Gemini 3.1 Pro**
- [x] Update `URL` constant in `src/ai/gemini.rs`
- [x] `cargo build` succeeds
- [x] Mark this task `[x]`
- [ ] git commit: `fix: update Gemini model to gemini-3.1-pro-preview`

---

### Task B2: Add AI usage tracking to backend `mark_paper` handler

**Files to modify:** `src/services/ai_marking.rs`, `src/db/database/tables/actions.rs` (or equivalent)
**Depends on:** none
**Parallel group:** P1

**Specification:**

After the Gemini API returns scores and the handler writes grades to the `grades` table, it must ALSO update the `aiusage` table for each student that was marked.

#### Step 1 — Upsert `aiusage` row per student

For each student in the marking batch, after writing the grade:

```sql
INSERT INTO aiusage (school, student, year, term, allocated, used, created, updated)
VALUES (?, ?, ?, ?, 0, 1, unixepoch('now'), unixepoch('now'))
ON CONFLICT (school, student, year, term)
DO UPDATE SET used = used + 1, updated = unixepoch('now');
```

This requires knowing `year` and `term` for the exam. The `MarkPaperRequest` has `school` and `exam` but not `year`/`term` directly. The handler must look up the exam's `year` and `term` from the `exams` table:

```sql
SELECT year, term FROM exams WHERE id = ? AND school = ?;
```

#### Step 2 — Append changelog entry for `aiusage`

After each upsert, append a changelog record so `watchChanges` streams the update to all clients:

```rust
// TBL_AIUSAGE = 24 (matches InsertData tag 24 and delta_writer case 24)
append_log(log_user, TBL_AIUSAGE as u8, OP_UPDATE, 0)?;
```

The `rowKey` format for the watch stream delta must be: `"{school}|{student}|{year}|{term}"` — pipe-delimited composite PK, matching the client's `_applyAiUsage()` parser.

#### Step 3 — Pre-check allocation (optional but recommended)

Before calling Gemini, optionally check if each student has remaining allocation:

```sql
SELECT allocated, used FROM aiusage WHERE school = ? AND student = ? AND year = ? AND term = ?;
```

If `used >= allocated` AND `allocated > 0`, skip that student and log a warning. If the row doesn't exist (first time), proceed — the insert in Step 1 will create it with `used = 1, allocated = 0`. A zero `allocated` means "unlimited" (no cap set by admin yet).

#### Step 4 — Wire into the spawned task

In `src/services/ai_marking.rs`, inside the `tokio::spawn` block that runs after Gemini returns, after the grade-writing loop, add the aiusage upsert loop. The function signature for the helper:

```rust
fn write_ai_usage(
    school: &str,
    student: i32,
    year: i32,
    term: i16,
) -> Result<()> {
    // 1. Upsert aiusage row (INSERT ON CONFLICT UPDATE used = used + 1)
    // 2. append_log(Id::system(), TBL_AIUSAGE, OP_UPDATE, 0)
    // 3. Return Ok(())
}
```

Also add the exam lookup at the start of the spawned task:

```rust
// Inside tokio::spawn, before the grade loop:
let (year, term) = fetch_exam_year_term(&school, &exam)?;
// Then after each grade write:
write_ai_usage(&school, score.adm, year, term)?;
```

**Backend constants to add:**
```rust
const TBL_AIUSAGE: u8 = 24;  // matches InsertData tag 24 and delta_writer case 24
```

**Update after completion:**
- [x] Add `write_ai_usage` helper function
- [x] Add `fetch_exam_year_term` helper function
- [x] Wire both into the `mark_paper` spawned task
- [x] Add `TBL_AIUSAGE` constant
- [x] `cargo build` succeeds
- [ ] Test: after AI marking, verify `aiusage` row exists with `used > 0`
- [x] Mark this task `[x]`
- [ ] git commit: `feat: track AI usage per student in aiusage table after marking`
