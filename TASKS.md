# TASKS.md

## Exam Paper PDF — Kenyan Format Improvements

---

### Task 01: PDF renderer — spacing, font sizes, right-aligned marks, answer lines, footer

**Files to create/modify:** `src/pdf.rs`
**Context files to read (if needed):** None — full spec below
**Depends on:** None
**Parallel group:** P1

**Specification:**

Five high-impact visual fixes — all pure renderer changes in `src/pdf.rs`.
Zero schema or proto changes required. Read `src/pdf.rs` in full before editing.
All changes are within `generate_paper_pdf`.

---

**Fix 1 — Increase font sizes**

Exam papers are printed on A4 — 10pt is too small to read comfortably.
- Question text body: currently `Pt(10.0)` -> change to `Pt(11.0)`
- Sub-header line (exam | subject | paper | grade): change to `Pt(12.0)`
- Inline marks annotation: keep at `Pt(11.0)` (same as body)
- Do NOT change school name (16pt) — already correct.

---

**Fix 2 — Question spacing**

Current gap between questions is ~3mm — far too tight for a printed exam paper.
Change the vertical gap inserted between questions (after one question's answer
lines and before the bold number of the next) to `12.0` mm.
The gap between question text and its answer lines (Fix 3) is separate: use `4.0` mm.

---

**Fix 3 — Answer lines below every question**

After rendering each question's text, draw ruled horizontal lines for the student
to write on. This is the most critical missing element — without it the paper
cannot be administered.

Number of lines: `max(3, question_marks * 2)` (minimum 3 lines, 2 lines per mark).

Each line:
- Full-width horizontal rule from left margin to right margin
- Stroke width: `0.3` pt
- Colour: RGB `(0.75, 0.75, 0.75)` light grey
- Vertical spacing between lines: `7.0` mm

Before drawing each line, check if remaining vertical space on the page is less
than `7.0` mm — if so, call the page-flush function first.

---

**Fix 4 — Right-aligned marks column**

Currently "(3 marks)" is appended inline at the end of the question text body.
Move it to the right margin on the same baseline as the first line of question text:

- Format: "({n} mark)" when n == 1, else "({n} marks)"
- Estimate text width: `marks_text.len() as f64 * font_size_pt * 0.5 * 0.3528` mm
- Place at `x = page_width_mm - right_margin_mm - text_width`, same `y` as the
  question number label
- Remove the inline "(X marks)" from the question text body

---

**Fix 5 — Page footer: page number + Turn over + END OF PAPER**

After every page is finalized, render a footer at the bottom margin:
- Page number: centered, format `"- {n} -"`, `Pt(9.0)`, regular font
- "Turn over": right-aligned, `Pt(9.0)`, regular font — on ALL pages except
  the last. Track page layers in a Vec; after all questions are done, iterate
  and add "Turn over" to all pages except the final one.

After the last question's answer lines and the total marks line, before closing
the final page, render:
- "— END OF PAPER —" centered, `Pt(11.0)`, bold, with `10.0` mm top margin

---

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Commit: `git add -A && git commit -m "fix: answer lines, spacing, font sizes, right-aligned marks, page footer in PDF renderer"`

---

### Task 02: PDF renderer — candidate info box + expanded instructions block

**Files to create/modify:** `src/pdf.rs`
**Context files to read (if needed):** Task 01 changes
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**

Both additions go between the existing second horizontal rule (after school name/motto)
and the first question. No schema changes required — content is templated.

---

**Addition 1 — Candidate information box**

Immediately after the second horizontal rule, render a bordered rectangle:
- Border: `0.5` pt, RGB `(0.3, 0.3, 0.3)`
- Internal padding: `4.0` mm on all sides
- Row height: `9.0` mm per row
- Five rows (label left-aligned at `Pt(10.0)`, regular; fill line is a horizontal
  rule at `y = row_baseline - 1.0 mm`, `0.4` pt, RGB `(0.6, 0.6, 0.6)`):
  1. `Name:` — fill line spans full remaining box width
  2. `Adm. No.:` — fill line: 60 mm
  3. `Class / Stream:` — fill line: 60 mm
  4. `Signature:` — fill line: 60 mm
  5. `Date:` — fill line: 40 mm
- After the box: `6.0` mm vertical gap before the instructions block

---

**Addition 2 — Expanded instructions block**

Replace the single hardcoded "Answer ALL questions." line with a 5-line
italic block at `Pt(10.0)`, italic font, left-aligned, `4.0` mm between lines:

1. "Answer ALL questions in this paper."
2. "Show all your working clearly in the spaces provided."
3. "All answers must be written in the spaces provided."
4. "Check that all pages are present before starting."
5. "Candidates should check the paper for any missing pages."

Keep the existing horizontal rule that currently follows the instructions —
ensure it still renders after the full 5-line block.

---

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Commit: `git add -A && git commit -m "feat: candidate info box and expanded instructions block in PDF header"`

---

### Task 03: Schema + proto — add time_allowed_minutes and instructions to papers table

**Files to create/modify:**
- New migration file in `migrations/`
- `src/models/paper.rs` (or equivalent — wherever the Paper struct is defined)
- `protos/question_bank.proto` — update `CreatePaperPayload`, `UpdatePaperPayload`,
  `PaperInsert`, and `FinalizePaperResponse`
- Regenerate and deliver updated Dart proto stubs:
  `question_bank.pb.dart` and `question_bank.pbgrpc.dart`

**Context files to read (if needed):** Existing papers migration, existing proto definitions
**Depends on:** None (can run in parallel with Tasks 01 and 02)
**Parallel group:** P1

**Specification:**

**Migration — add two nullable columns to `papers`:**

```sql
ALTER TABLE papers ADD COLUMN time_allowed_minutes SMALLINT;
ALTER TABLE papers ADD COLUMN instructions TEXT;
```

`time_allowed_minutes` — e.g. 90 for 1h30m, 150 for 2h30m. NULL means not set.
`instructions` — free-text custom instructions, newline-separated. NULL = use defaults.

**Update Paper model** (Rust struct) to include:
```rust
pub time_allowed_minutes: Option<i16>,
pub instructions: Option<String>,
```

**Update proto messages — add optional fields to:**

`CreatePaperPayload` and `UpdatePaperPayload`:
```proto
optional int32  time_allowed_minutes = <next_field_number>;
optional string instructions         = <next_field_number>;
```

`PaperInsert` (used in SyncDelta / WatchChanges):
```proto
optional int32  time_allowed_minutes = <next_field_number>;
optional string instructions         = <next_field_number>;
```

The `finalize_paper` handler reads these directly from the DB — no change to
`FinalizePaperResponse` needed for this task.

Regenerate Dart stubs and deliver updated files to the client team.

---

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Commit: `git add -A && git commit -m "feat: add time_allowed_minutes and instructions to papers schema and proto"`

---

### Task 04: PDF renderer — wire time_allowed + total marks + custom instructions into header

**Files to create/modify:**
- `src/pdf.rs`
- `src/services/question_bank.rs` (finalize_paper handler)

**Context files to read (if needed):** Task 03 changes
**Depends on:** Task 03, Task 01
**Parallel group:** P3

**Specification:**

**In `finalize_paper` handler (`src/services/question_bank.rs`):**
- After fetching the paper record, extract `time_allowed_minutes: Option<i16>`
  and `instructions: Option<String>`
- Pass them into `generate_paper_pdf`

**Update `generate_paper_pdf` signature in `src/pdf.rs`:**

```rust
pub fn generate_paper_pdf(
    school_name: &str,
    school_motto: Option<&str>,
    exam_name: &str,
    subject_name: &str,
    paper_num: i32,
    grade: i16,
    time_allowed_minutes: Option<i16>,
    custom_instructions: Option<&str>,
    questions: &[(String, i16, Vec<(String, i16)>)],
) -> Vec<u8>
```

**Time allowed rendering** — add after the exam/subject/grade sub-header line:
- If Some and >= 60 minutes: `"Time: {h} hour(s) {rem} minutes"` (omit remainder if 0)
- If Some and < 60: `"Time: {mins} minutes"`
- If None: render nothing

Font: `Pt(11.0)`, regular, centered.

**Total marks** — always rendered (no schema change needed):
`"Total Marks: {sum}"` where sum = all question marks added together.
Font: `Pt(11.0)`, regular, centered. Render on the line below time allowed,
or in its place if time_allowed is None.

**Custom instructions** — in the instructions block added in Task 02:
- If `custom_instructions` is Some: split on newline, render each line instead
  of the 5 default lines
- If None: use the default 5-line block from Task 02

---

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Commit: `git add -A && git commit -m "feat: time allowed, total marks, and custom instructions in PDF header"`

---

### Task 05: Generate separate marking scheme PDF from existing rubric data

**Files to create/modify:**
- `src/pdf.rs` — add `generate_marking_scheme_pdf` function
- `src/services/question_bank.rs` — call it in `finalize_paper`, upload to S3
- `protos/question_bank.proto` — add `marking_scheme_url` and
  `marking_scheme_expiry` fields to `FinalizePaperResponse`
- Regenerate and deliver updated Dart proto stubs

**Context files to read (if needed):** Task 01 changes (shared helper functions)
**Depends on:** Task 01
**Parallel group:** P2

**Specification:**

The rubric data is already passed into `generate_paper_pdf` as the third tuple
element `Vec<(String, i16)>` (criterion text + marks) but is currently
destructured as `_rubric` and ignored. This task generates a second PDF from it.

**New function `generate_marking_scheme_pdf`:**

Same signature as `generate_paper_pdf` but produces a marking scheme document.

Header: identical to the question paper header EXCEPT append " — MARKING SCHEME"
to the exam title line, e.g. "END OF TERM 1 EXAMINATIONS — MARKING SCHEME".

For each question (1-based, using enumerate + 1):
- Bold question number: "{n}." at `Pt(11.0)`, bold
- Question text (first 120 chars + "…" if longer): `Pt(10.0)`, italic
- For each rubric criterion (criterion_text, criterion_marks):
  - Bullet left-aligned: "    • {criterion_text}" at `Pt(10.0)`, regular
  - Marks right-aligned at right margin: "[{n} mark(s)]" at `Pt(10.0)`
  - Vertical gap between criteria: `3.0` mm
- After all criteria: thin full-width divider at `0.3` pt, RGB (0.8, 0.8, 0.8)
- Total right-aligned: "Total: {sum} mark(s)" at `Pt(10.0)`, bold
- Vertical gap between questions: `8.0` mm

If a question has an empty rubric Vec, render:
"    (No marking criteria defined)" in italic at `Pt(10.0)`.

Footer: same page number rendering as the question paper (Task 01, Fix 5).
Final line: "— END OF MARKING SCHEME —" centered, `Pt(11.0)`, bold.

**In `finalize_paper` handler:**
1. Call `generate_marking_scheme_pdf` after `generate_paper_pdf`
2. Upload to S3 with key pattern:
   `marking_schemes/{school}/{exam}/{subject}/{paper}/{grade}/{stream_or_0}.pdf`
3. Generate a presigned GET URL with the same expiry as the question paper PDF
4. Return in `FinalizePaperResponse`:
   ```proto
   string marking_scheme_url    = <next_field_number>;
   int64  marking_scheme_expiry = <next_field_number>;
   ```

Regenerate Dart stubs and deliver updated files to the client team.

---

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Commit: `git add -A && git commit -m "feat: generate and serve separate marking scheme PDF from rubric data"`

---

### Task 06: Schema + proto — section support on paper_questions

**Files to create/modify:**
- New migration file in `migrations/`
- `src/models/paper_question.rs` (or equivalent)
- `protos/question_bank.proto` — update `PaperQuestion` message, add
  `SetPaperQuestionSection` RPC with request and response messages
- Regenerate and deliver updated Dart proto stubs

**Context files to read (if needed):** Existing paper_questions migration, existing proto
**Depends on:** None (parallel with anything)
**Parallel group:** P1

**Specification:**

**Migration:**
```sql
ALTER TABLE paper_questions ADD COLUMN section TEXT;
-- Valid values: 'A', 'B', 'C', 'D', or NULL (unsectioned)
```

**Update `PaperQuestion` proto message — add optional field:**
```proto
optional string section = <next_field_number>;
```

Populate `section` from the DB in both `generate_paper` and `get_paper_questions`
response builders. Initially all rows are NULL so all questions return with no section.

**New RPC — `SetPaperQuestionSection`:**

```proto
rpc SetPaperQuestionSection (SetPaperQuestionSectionRequest)
    returns (SetPaperQuestionSectionResponse);

message SetPaperQuestionSectionRequest {
  string         school   = 1;
  string         exam     = 2;
  int32          subject  = 3;
  optional int32 paper    = 4;
  int32          grade    = 5;
  optional int32 stream   = 6;
  int32          position = 7;
  optional string section = 8;
}

message SetPaperQuestionSectionResponse {}
```

Handler logic:
- Validate section value is one of "A", "B", "C", "D", or absent (clears to NULL)
- Validate caller is a member of the school with appropriate permissions
- UPDATE paper_questions SET section = ? WHERE school = ? AND exam = ? AND
  subject = ? AND paper = ? AND grade = ? AND stream matches AND position = ?
- Return NOT_FOUND if no row matched
- Return INVALID_ARGUMENT if section value is not in the allowed set

Regenerate Dart stubs and deliver updated files to the client team.

---

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Commit: `git add -A && git commit -m "feat: section column on paper_questions, SetPaperQuestionSection RPC"`

---

### Task 07: PDF renderer — section headers

**Files to create/modify:**
- `src/pdf.rs`
- `src/services/question_bank.rs` (finalize_paper — pass section data)

**Context files to read (if needed):** Task 06 changes, Task 01 changes
**Depends on:** Task 06, Task 01
**Parallel group:** P4

**Specification:**

Update the questions parameter of `generate_paper_pdf` to carry section data:

```rust
// Before:
questions: &[(String, i16, Vec<(String, i16)>)]
//           text   marks  rubric

// After:
questions: &[(String, i16, Vec<(String, i16)>, Option<String>)]
//           text   marks  rubric               section
```

Update `finalize_paper` handler to populate the section from `paper_questions.section`.

**Section header rendering logic:**

Track `current_section: Option<&str> = None` before the question loop.
Before rendering each question, check if its section differs from `current_section`.
If it does, update `current_section` and (if the new value is Some) render a
section header block:

1. Vertical gap: `8.0` mm before the header
2. "SECTION {letter}" — `Pt(13.0)`, bold, centered
3. Section marks total: sum the marks of all questions sharing this section value,
   render as "({total} marks)" — `Pt(11.0)`, regular, centered, same line or next
4. Instruction line: "Answer ALL questions in this section." — `Pt(10.0)`, italic, centered
5. Full-width horizontal rule: `0.5` pt, RGB (0.4, 0.4, 0.4)
6. Vertical gap: `6.0` mm before the first question of the section

**Question numbering:** reset to 1 at the start of each section. Track a
`section_question_index: usize` that resets whenever `current_section` changes.
Use this (plus 1) as the display number label instead of the global enumerate index.

**Fallback:** If ALL questions have `section = None`, output is identical to
the pre-Task-07 output — no section headers are inserted.

---

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Commit: `git add -A && git commit -m "feat: section headers in PDF renderer"`
