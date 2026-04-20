# TASKS.md

## Question Bank Server Audit — Global Question Import + Accurate Error Mapping

The Flutter client reported two issues while importing questions from the system dashboard:

1. Importing many JSON files fails with `school not found`.
2. 8-4-4 question/topic selection on the client only exposed Form 3 and Form 4.

Server-side examination findings:
- Question-bank CRUD/import/list is already modeled as **global/system-wide**, not school-scoped.
- `CreateQuestionRequest`, `BulkImportRequest`, `ListQuestionsRequest`, `GetQuestionRequest`, `UpdateQuestionRequest`, and `DeleteQuestionRequest` in `protos/services/question_bank.proto` have **no `school` field**.
- The `questions` table has **no `school` column**.
- The `school not found` message is currently caused by **misleading error mapping** in `src/services/question_bank.rs`: bulk import returns `Error::SchoolNotFound` when the real failure is `subject not found` or `topic not found`.
- No server-side restriction was found limiting 8-4-4 question-bank grades to Form 3/Form 4. That restriction appears to be client-side only.
- Bulk import curriculum parsing is currently too permissive: `"844"` maps to 8-4-4 and every other string silently maps to CBC.

These tasks focus the server work on preserving the global question-bank contract, fixing misleading errors, and tightening validation/documentation.

---

### Task 01: Replace misleading `SchoolNotFound` mapping in question-bank bulk import
**Files to create/modify:** `src/services/question_bank.rs`, `src/types/error.rs`
**Context files to read (if needed):** `AGENT.md`
**Depends on:** none
**Parallel group:** P1

**Specification:**
In `src/services/question_bank.rs`, the `bulk_import_questions` flow currently logs:
- `bulk_import: subject not found: ...`
- `bulk_import: topic not found: ...`

but then returns `Error::SchoolNotFound`, which becomes the gRPC message `school not found` via `src/types/error.rs`.

Fix this by introducing accurate error variants in `src/types/error.rs` and using them from `bulk_import_questions`.

Required behavior:
- Missing subject during bulk import must return a subject-specific not-found error.
- Missing topic during bulk import must return a topic-specific not-found error.
- The gRPC status message must no longer say `school not found` for these catalog lookup failures.
- Preserve the existing not-found semantics (`Status::not_found`) unless there is a strong reason to use another code.

Suggested error variants:
- `SubjectNotFound`
- `TopicNotFound`

Also update the `From<Error> for tonic::Status` mapping so these new variants produce clear client-visible messages such as:
- `subject not found`
- `topic not found`

Do not change school-scoped paper-generation/marking endpoints in this task.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task 02: Tighten bulk-import curriculum validation instead of silently coercing unknown values to CBC
**Files to create/modify:** `src/services/question_bank.rs`, `src/types/error.rs`
**Context files to read (if needed):** `AGENT.md`
**Depends on:** none
**Parallel group:** P1

**Specification:**
In `src/services/question_bank.rs`, `bulk_import_questions` currently maps curriculum with logic equivalent to:
- `"844"` => 8-4-4
- everything else => CBC

This is too permissive and can hide malformed import payloads.

Change the validation so that only explicit accepted values are allowed.

Required behavior:
- Accept `"844"` for 8-4-4.
- Accept `"cbc"` for CBC.
- Reject any other curriculum string with a clear client-visible error.

Implementation requirements:
- Add or reuse an error variant in `src/types/error.rs` that maps to `Status::invalid_argument`.
- The error message should clearly indicate the accepted values, e.g. `invalid curriculum: expected \"844\" or \"cbc\"`.
- Keep the rest of the bulk import flow unchanged.

Do not add school scoping to question-bank import.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task 03: Document and preserve the global question-bank contract in proto comments and service code
**Files to create/modify:** `protos/services/question_bank.proto`, `src/services/question_bank.rs`
**Context files to read (if needed):** `AGENT.md`
**Depends on:** Tasks 01–02
**Parallel group:** P2

**Specification:**
Add documentation comments that make the contract explicit:
- Question-bank CRUD/import/list endpoints are **global catalog operations**.
- They are **not school-scoped**.
- School-scoped fields belong only to paper assembly / grading / marking endpoints.

Required updates:
1. In `protos/services/question_bank.proto`, add comments above these request/messages or RPCs clarifying they are global/system-wide:
   - `CreateQuestionRequest`
   - `UpdateQuestionRequest`
   - `DeleteQuestionRequest`
   - `GetQuestionRequest`
   - `BulkImportRequest`
   - `ListQuestionsRequest`
2. Add comments near the school-scoped paper/marking requests clarifying that those are intentionally school-bound:
   - `GeneratePaperRequest`
   - `RegenerateQuestionRequest`
   - `FinalizePaperRequest`
   - `GetPaperPdfRequest`
   - `GetQuestionGradesRequest`
   - `MarkingStatusRequest`
3. In `src/services/question_bank.rs`, add a short comment in the bulk import / create / list section noting that subject/topic/question catalog operations are global and must not derive or require a school.

Do not change request shapes or add a `school` field.

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task 04: Verify no server-side 8-4-4 Form 3/Form 4 restriction exists in question-bank/catalog flows
**Files to create/modify:** `TASKS.md`
**Context files to read (if needed):** `AGENT.md`
**Depends on:** none
**Parallel group:** P1

**Specification:**
This is a documentation-only verification task.

Reconfirm in the server code that question-bank/catalog flows do not restrict 8-4-4 to only Form 3/Form 4, and append a short findings note under this task in `TASKS.md` with:
- files checked,
- whether any restriction was found,
- and whether any server change is required for 8-4-4 grade coverage.

Expected current conclusion unless new evidence is found:
- No server-side Form 3/Form 4 restriction exists in the relevant question-bank/catalog code.
- No server change is required for the 8-4-4 grade-coverage issue; that issue is client-side.

**Findings note:**
- Files checked: `src/services/question_bank.rs`, `protos/services/question_bank.proto`
- Restriction found: none in the server-side question-bank/catalog flows reviewed
- Evidence: bulk import only resolves `(subject, curriculum, grade, topic)` catalog records; create/list/get question flows operate on `topic_id`/`question_id`; paper-generation and marking requests accept a generic `grade` integer and do not special-case 8-4-4 Form 3/Form 4
- Server change required for 8-4-4 grade coverage: no; the reported Form 3/Form 4 limitation is not enforced by the reviewed server code and remains client-side

**Update after completion:**
- [x] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task
