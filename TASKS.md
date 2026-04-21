# TASKS.md — Ledger (Server)

## Feature Group P — Paper Generation: Clear/Regenerate, Multi-Stream Copy

---

### Task S01: Add `ClearPaperQuestions` RPC to `QuestionBank` gRPC service

**Proto file to modify:** `question_bank.proto`
**Server handler to create/modify:** QuestionBank service implementation
**Client stubs to regenerate after implementation:** Dart stubs at
  `eduxal/lib/proto/services/question_bank.pb.dart` and
  `eduxal/lib/proto/services/question_bank.pbgrpc.dart`
**Depends on:** None
**Parallel group:** S1 (can run alongside S02)

---

#### Proto additions

Add the following message types and RPC to `question_bank.proto`:

```protobuf
// ─── ClearPaperQuestions ─────────────────────────────────────────────────────

// Request to delete all generated questions for a paper and invalidate its PDF.
// Only allowed when papers.status = Pending (0). Calling this on a paper that
// has already started (Progress, Done, Marked) returns FAILED_PRECONDITION.
message ClearPaperQuestionsRequest {
  string         school  = 1;  // school UUID
  string         exam    = 2;  // exam UUID
  int32          subject = 3;  // subjects.id (global catalog integer)
  optional int32 paper   = 4;  // papers.paper (null = unnumbered paper)
  int32          grade   = 5;  // papers.grade
  optional int32 stream  = 6;  // papers.stream (null = grade-wide paper)
}

message ClearPaperQuestionsResponse {
  int32 questions_deleted = 1;  // count of paper_questions rows deleted
  bool  pdf_deleted       = 2;  // true if an S3 PDF was found and deleted
}
```

Add to the `QuestionBank` service block:

```protobuf
// Delete all generated questions and the generated PDF for a paper.
// The paper must be in Pending status. Returns FAILED_PRECONDITION otherwise.
rpc ClearPaperQuestions(ClearPaperQuestionsRequest) returns (ClearPaperQuestionsResponse);
```

---

#### Server implementation

**Permission check:**
The caller must satisfy ANY of the following:
- Has `Exams.Update` permission (bit 2 of the Exams resource bitmask in their scopes)
- Is the creator of the exam (`exams.teacher = caller_id`)
- Is the invigilator of the paper (`papers.invigilator = caller_id`)

If none apply → return `PERMISSION_DENIED`.

**Validation:**
1. Fetch the `papers` row for `(school, exam, subject, paper, grade, stream)`.
   - If not found → return `NOT_FOUND`.
   - If `papers.status != 0` (not Pending) → return `FAILED_PRECONDITION` with message:
     "Cannot clear questions for a paper that has already started."

**Deletion — database:**
2. Delete all rows from the `paper_questions` table (or equivalent server-side question
   linking table) where the composite key matches:
   `(school = ?, exam = ?, subject = ?, paper IS ? or paper = ?, grade = ?, stream IS ? or stream = ?)`.
   Capture the deleted row count as `questions_deleted`.

**Deletion — S3:**
3. Attempt to delete the following S3 objects (use your S3 client; suppress
   "key does not exist" errors — set `pdf_deleted = true` only if at least one
   of these objects actually existed and was deleted):
   - Question paper PDF:
     `schools/{school}/exams/{exam}/{subject}/{paper_or_0}/question_paper_{grade}_{stream_or_0}.pdf`
   - Marking scheme PDF:
     `schools/{school}/exams/{exam}/{subject}/{paper_or_0}/marking_scheme_{grade}_{stream_or_0}.pdf`

   Where `{paper_or_0}` = `papers.paper` value, or `0` when `papers.paper IS NULL`.
   Where `{stream_or_0}` = `papers.stream` value, or `0` when `papers.stream IS NULL`.

**Response:**
4. Return `ClearPaperQuestionsResponse { questions_deleted, pdf_deleted }`.

**Side effects:**
- Do NOT change `papers.status`.
- Do NOT write a `SyncDelta` or `server_logs` entry. This operation does not need to
  propagate to other clients — the paper row itself is unchanged.

---

#### Dart stub regeneration

After implementation is complete, regenerate the Dart client stubs using `protoc` with the
Dart plugin. The client team needs the updated versions of:
- `eduxal/lib/proto/services/question_bank.pb.dart`
- `eduxal/lib/proto/services/question_bank.pbgrpc.dart`
- `eduxal/lib/proto/services/question_bank.pbjson.dart`

Provide these files to the client team before they can complete client Task P03.

**Completion checklist:**
- [x] Mark this task `[x]`
- [ ] Provide updated Dart stubs to client team
- [ ] Commit: `feat: add ClearPaperQuestions RPC to QuestionBank service`

---

### Task S02: Add `CopyPaperToStreams` RPC to `QuestionBank` gRPC service

**Proto file to modify:** `question_bank.proto`
**Server handler to create/modify:** QuestionBank service implementation
**Client stubs to regenerate after implementation:** Dart stubs (same as S01)
**Depends on:** None
**Parallel group:** S1 (can run alongside S01)

---

#### Proto additions

Add the following message types and RPC to `question_bank.proto`:

```protobuf
// ─── CopyPaperToStreams ───────────────────────────────────────────────────────

// Copy the generated question set from one stream to one or more additional
// streams in the same grade. Each target stream gets its own PDF generated.
// The source stream must already have generated questions (via GeneratePaper).
// Papers.status is NOT changed for source or target streams — all remain Pending.
message CopyPaperToStreamsRequest {
  string         school          = 1;  // school UUID
  string         exam            = 2;  // exam UUID
  int32          subject         = 3;  // subjects.id
  optional int32 paper           = 4;  // papers.paper (null = unnumbered)
  int32          grade           = 5;  // papers.grade
  optional int32 source_stream   = 6;  // stream that already has questions
  repeated int32 target_streams  = 7;  // streams to copy to (1–10 items)
}

// Result for one target stream.
message StreamCopyResult {
  int32  stream                = 1;  // the target stream code from the request
  bool   success               = 2;  // false if this stream's copy failed
  string pdf_url               = 3;  // presigned GET URL (empty if success=false)
  int64  pdf_expiry            = 4;  // seconds since epoch (0 if success=false)
  string marking_scheme_url    = 5;  // presigned GET URL; empty if not available
  int64  marking_scheme_expiry = 6;  // seconds since epoch; 0 if not available
  string error                 = 7;  // human-readable error; empty if success=true
}

message CopyPaperToStreamsResponse {
  repeated StreamCopyResult results = 1;
}
```

Add to the `QuestionBank` service block:

```protobuf
// Copy a generated paper to additional streams in the same grade.
// Each target stream gets copied question rows and a freshly generated PDF.
// Partial failures are allowed — failed streams return success=false with an
// error message; successful streams are unaffected.
rpc CopyPaperToStreams(CopyPaperToStreamsRequest) returns (CopyPaperToStreamsResponse);
```

---

#### Server implementation

**Permission check:**
Same as `ClearPaperQuestions` — `Exams.Update` OR exam creator OR invigilator.

**Validation:**
1. Validate `target_streams` is non-empty and has ≤10 items.
   If violated → return `INVALID_ARGUMENT`.
2. Validate no element in `target_streams` equals `source_stream`.
   If violated → return `INVALID_ARGUMENT` with message "target_streams must not include source_stream".
3. Fetch source paper_questions for `(school, exam, subject, paper, grade, source_stream)`.
   If the source has zero questions → return `FAILED_PRECONDITION` with message:
   "Source paper has no generated questions. Generate questions first."

**Processing — for each target stream (can be done in parallel internally):**

For each `target_stream` in `target_streams`:

a. **Clear existing questions** for `(school, exam, subject, paper, grade, target_stream)` if any
   (same deletion logic as `ClearPaperQuestions`, but skip S3 PDF deletion for now — the
   finalization step in (d) will overwrite them).

b. **Copy question rows**: Insert copies of all source `paper_questions` rows, substituting
   `target_stream` for the stream field. Preserve `question_id`, `order`, `marks`, `section`,
   and any other columns. Assign new server-generated row IDs to the copies.

c. **Ensure a `papers` row exists** for `(school, exam, subject, paper, grade, target_stream)`:
   - If it already exists → leave it unchanged (do not alter its status or any other field).
   - If it does not exist → insert a new `papers` row copying all fields from the source paper
     (`invigilator`, `start`, `end`, `topic`, `time_allowed_minutes`, `custom_instructions`)
     and setting `status = 0` (Pending), `grade = grade`, `stream = target_stream`.
   - When inserting a new `papers` row, also write a `server_logs` entry so the `WatchChanges`
     stream delivers a `PaperInsert` `SyncDelta` to all connected clients for this school.

d. **Run PDF finalization** for the target stream — use exactly the same pipeline as
   `FinalizePaper` (question ordering, LaTeX/HTML generation, S3 upload, presigned URL
   generation). Return presigned GET URLs (valid ~1 month) for the PDF and marking scheme.

e. **On success**: append `StreamCopyResult { stream: target_stream, success: true,
   pdf_url, pdf_expiry, marking_scheme_url, marking_scheme_expiry }`.

f. **On any error** for a specific stream: catch the exception, append
   `StreamCopyResult { stream: target_stream, success: false, error: <message> }`,
   and continue to the next stream. Do NOT abort the entire request.

**Response:**
Return `CopyPaperToStreamsResponse { results }` after all target streams are processed.

**Status invariant:**
Do NOT change `papers.status` for any stream (source or target). All papers remain Pending.

**Performance note:**
PDF generation per stream may take 5–30 seconds depending on question count.
The client uses a 120-second deadline. If generating many streams concurrently,
consider parallel processing with a concurrency limit of 3–5 streams at a time.

---

#### Dart stub regeneration

Same as S01 — provide updated Dart stubs to client team after implementation.
The client needs `CopyPaperToStreamsRequest`, `CopyPaperToStreamsResponse`, and
`StreamCopyResult` message types, plus the new `CopyPaperToStreams` RPC method
in `QuestionBankClient`.

**Completion checklist:**
- [x] Mark this task `[x]`
- [ ] Provide updated Dart stubs to client team
- [ ] Commit: `feat: add CopyPaperToStreams RPC to QuestionBank service`
