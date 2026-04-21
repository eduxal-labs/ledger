# TASKS.md

## Bug Fixes — Paper Columns + Question Marks Overshoot

---

### [x] Task S01: Fix `insert_paper` to persist `time_allowed_minutes` and `instructions`

**File:** `src/db/database/tables/insert.rs`
**Depends on:** None
**Parallel group:** P1

**Root cause:** `insert_paper` builds an INSERT statement that lists 13 columns but omits the two new columns added in the T03 migration (`time_allowed_minutes`, `instructions`). Values from the proto payload in `PaperInsert` are silently discarded — the DB always stores NULL for these fields even when the client sends values.

**Fix:** Update the `insert_paper` function (line ~301) to include both new columns in the INSERT:

```sql
INSERT INTO papers (
    school, exam, subject, paper, topic, invigilator, start, "end",
    status, grade, stream, time_allowed_minutes, instructions,
    created, updated
)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
```

Add the two new binds after `stream`:
```rust
.bind::<Nullable<SmallInt>, _>(row.time_allowed_minutes.map(|v| v as i16))
.bind::<Nullable<Text>, _>(row.instructions.as_deref())
```

The `PaperInsert` struct already has both fields (verify: `row.time_allowed_minutes: Option<i32>`, `row.instructions: Option<String>`).

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Commit: `git add -A && git commit -m "fix: persist time_allowed_minutes and instructions in insert_paper"`

---

### [x] Task S02: Fix `fetch_paper` column list — causes "internal server error" in client notifications

**File:** `src/db/database/tables/actions.rs`
**Depends on:** None
**Parallel group:** P1

**Root cause:** `fetch_paper` (line ~863) runs:
```sql
SELECT school, exam, subject, paper, topic, invigilator, start, "end",
       status, grade, stream, created, updated
FROM papers WHERE ...
```
This selects 13 columns. However, `PaperRow` (in `rows.rs`) now has **15** fields including `time_allowed_minutes` and `instructions`. Diesel's `sql_query` deserializes columns positionally — the result has fewer columns than the struct expects, causing a deserialization failure → `Error::Internal` → sync engine receives error code 3 with message `"Validation error: internal server error"` → client notification fires.

**Fix:** Add the two missing columns to the SELECT in `fetch_paper`:

```sql
SELECT school, exam, subject, paper, topic, invigilator, start, "end",
       status, grade, stream, created, updated,
       time_allowed_minutes, instructions
FROM papers WHERE school = ? AND exam = ? AND subject = ?
AND paper IS ? AND grade = ? AND stream IS ?
```

No other changes needed — `PaperRow` already has the fields with correct Diesel types.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Commit: `git add -A && git commit -m "fix: add time_allowed_minutes/instructions to fetch_paper SELECT — fixes internal server error on createPaper"`

---

### [x] Task S03: Fix `update_paper` to update `time_allowed_minutes` and `instructions`

**File:** `src/db/database/tables/update.rs`
**Depends on:** None
**Parallel group:** P1

**Root cause:** `update_paper` (line ~474) builds:
```sql
UPDATE papers SET
  topic = COALESCE(?, topic),
  invigilator = COALESCE(?, invigilator),
  start = COALESCE(?, start),
  "end" = COALESCE(?, "end"),
  status = COALESCE(?, status),
  updated = ?
WHERE school = ? AND exam = ? AND subject = ? AND paper IS ? AND grade = ? AND stream IS ?
```
The `time_allowed_minutes` and `instructions` fields from `UpdatePaperPayload` are never written to the DB.

**Fix:** Extend the SET clause (before `updated = ?`):
```sql
time_allowed_minutes = COALESCE(?, time_allowed_minutes),
instructions = COALESCE(?, instructions),
```

Add the two binds after the existing `status` bind and before the `updated` bind:
```rust
.bind::<Nullable<SmallInt>, _>(row.time_allowed_minutes.map(|v| v as i16))
.bind::<Nullable<Text>, _>(row.instructions.as_deref())
```

`UpdatePaperPayload` already has `time_allowed_minutes: Option<i32>` and `instructions: Option<String>` — verify field names match.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Commit: `git add -A && git commit -m "fix: update time_allowed_minutes/instructions in update_paper"`

---

### [x] Task S04: Fix `SQL_PAPERS` snapshot query — same missing columns bug

**File:** `src/db/database/tables/snapshot.rs`
**Depends on:** None
**Parallel group:** P1

**Root cause:** Line ~394:
```rust
const SQL_PAPERS: &str = "SELECT school, exam, subject, paper, topic, invigilator, start, \"end\", status, grade, stream, created, updated FROM papers";
```
This 13-column SELECT is used for the initial sync snapshot. When Diesel deserializes results into `PaperRow` (15 fields), it fails → `Error::Internal` during watch/sync → clients cannot sync paper data.

**Fix:** Add the two missing columns at the end of `SQL_PAPERS`:
```rust
const SQL_PAPERS: &str = "SELECT school, exam, subject, paper, topic, \
    invigilator, start, \"end\", status, grade, stream, created, updated, \
    time_allowed_minutes, instructions FROM papers";
```

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Commit: `git add -A && git commit -m "fix: add time_allowed_minutes/instructions to SQL_PAPERS snapshot query"`

---

### [x] Task S05: Fix `select_random_questions` marks overshoot — generates 53 marks instead of 30

**File:** `src/db/database/tables/question_bank.rs`
**Depends on:** None
**Parallel group:** P1

**Root cause:** The current greedy fill algorithm (line ~237):
```rust
for row in rows {
    if current_marks >= target_marks as i32 { break; }
    current_marks += row.marks as i32;  // adds unconditionally
    selected.push(row);
}
```
This adds a question **even when it pushes the total well past the target**. The break fires on the NEXT iteration after overshooting. Result: multiple questions can push the total far beyond the target (e.g. target=30, actual=53).

**Fix:** Replace the loop body with a non-overshooting fill that tracks the best fallback:

```rust
let mut selected = Vec::new();
let mut current_marks = 0i32;
let mut best_fallback: Option<QuestionRow> = None;

for row in rows {
    if current_marks >= target_marks as i32 {
        break;
    }
    let new_total = current_marks + row.marks as i32;
    if new_total <= target_marks as i32 {
        // Fits within remaining budget — add it
        current_marks = new_total;
        selected.push(row);
    } else {
        // Would overshoot — remember the smallest overshooting option as fallback
        if best_fallback.is_none()
            || row.marks < best_fallback.as_ref().unwrap().marks
        {
            best_fallback = Some(row);
        }
    }
}

// If we haven't reached the target, add the smallest overshooting question
// as a last resort (this preserves the original contract: selected_marks >= target
// when enough questions exist).
if current_marks < target_marks as i32 {
    if let Some(fb) = best_fallback {
        current_marks += fb.marks as i32;
        selected.push(fb);
    }
}
```

**Key properties of the fix:**
- Total marks will always be ≤ target unless no single question fits (in which case the smallest overshooting question is used as fallback)
- Randomisation is preserved (ORDER BY RANDOM() in the SQL query)
- The downstream check `if selected_marks < alloc.marks { return Err(NotEnoughQuestions) }` is still respected

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Commit: `git add -A && git commit -m "fix: non-overshooting question selection in select_random_questions"`

---

### [x] Task S06: Fix `get_marking_status` — returns QUEUED when no job exists

**File:** `src/services/question_bank.rs`
**Depends on:** None
**Parallel group:** P1

**Root cause:** When no marking job row exists for a paper, the handler returns:
```rust
None => Ok::<_, Error>(MarkingStatusResponse {
    phase: 0, // QUEUED default when no entry exists
    ...
})
```
Phase 0 = `QUEUED`. The client's `watchMarkingStatus` stream only stops when `complete` or `failed` is received — it polls forever on `QUEUED`. This causes the client to permanently show "Queued for marking..." for papers that have never had marking triggered.

**Fix:** Return `Error::NotFound` when no marking row exists, so the client's `getMarkingStatus` call returns an `Err` result and the watch stream terminates cleanly:

```rust
None => Err(Error::NotFound),
```

The client's `watchMarkingStatus` already handles `Err` by yielding `MarkingPhase.failed` and stopping the stream. A separate client-side fix (Task C01) prevents the indicator from appearing for papers that don't have marking triggered.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Commit: `git add -A && git commit -m "fix: return not_found instead of QUEUED when no marking job exists"`

