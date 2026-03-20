# TASKS.md
---

## Feature: File Sync — Marking Schemes & Answer Sheets (Server)

### Overview

The client stores marking scheme images and student answer sheet images locally. They need to sync across devices via the existing push/watch gRPC streams. This requires:

1. New proto messages for 4 new sync actions.
2. Two new server tables (`scheme_pages`, `answer_pages`) to track S3 file metadata.
3. Action handlers that generate presigned S3 URLs and create metadata rows.
4. Changelog/watch integration so other clients receive deltas with download URLs.

### Coordination

**S1 (proto definitions) is BLOCKING.** After S1 is complete:
- Notify the user so the client agent can regenerate proto stubs (eduxal Task C2).
- Then S2–S5 (server) and C2–C8 (client) run in parallel.

### FileUrl.path Convention

The `FileUrl.path` field must use **relative paths** that the client resolves against its `appDir`:

- Scheme: `submissions/{school}/{exam}/{subject}_{paper}/scheme/{page}.jpg`
- Answer: `submissions/{school}/{exam}/{subject}_{paper}/{adm}/{page}.jpg`

Where `{paper}` is the paper number (use `0` when paper is NULL / single-paper subject).

---

### Task S1: Proto definitions for file sync actions (BLOCKING)

**Files to modify:** `proto/services/sync.proto` (or equivalent `.proto` source)
**Depends on:** none
**Parallel group:** — (must complete before all other tasks)

**Specification:**

Add the following to `sync.proto`:

#### New payload messages:

```protobuf
// Scheme upload — replace/set scheme pages for a paper.
// Server deletes existing pages, creates new rows, returns PUT URLs.
message UploadSchemePayload {
  string school = 1;
  string exam = 2;
  int32 subject = 3;
  optional int32 paper = 4;  // nullable — NULL means single-paper subject
  int32 count = 5;           // number of pages being uploaded (>= 1)
}

// Scheme delete — remove all scheme pages for a paper.
message DeleteSchemePayload {
  string school = 1;
  string exam = 2;
  int32 subject = 3;
  optional int32 paper = 4;
}

// Answer sheet upload — replace/set answer pages for a student's paper.
message UploadAnswerSheetPayload {
  string school = 1;
  string exam = 2;
  int32 student = 3;
  int32 subject = 4;
  optional int32 paper = 5;
  int32 count = 6;
}

// Answer sheet delete — remove all answer pages for a student's paper.
message DeleteAnswerSheetPayload {
  string school = 1;
  string exam = 2;
  int32 student = 3;
  int32 subject = 4;
  optional int32 paper = 5;
}
```

#### New InsertData messages (for watch stream):

```protobuf
message SchemePageInsert {
  string school = 1;
  string exam = 2;
  int32 subject = 3;
  optional int32 paper = 4;
  int32 page = 5;
  string key = 6;       // S3 object key
  int64 created = 7;
}

message AnswerPageInsert {
  string school = 1;
  string exam = 2;
  int32 student = 3;
  int32 subject = 4;
  optional int32 paper = 5;
  int32 page = 6;
  string key = 7;
  int64 created = 8;
}
```

#### Extend ActionRequest oneof:

Add new payload variants to the `ActionRequest` oneof (use next available tag numbers):

```protobuf
UploadSchemePayload upload_scheme = <next_tag>;
DeleteSchemePayload delete_scheme = <next_tag>;
UploadAnswerSheetPayload upload_answer_sheet = <next_tag>;
DeleteAnswerSheetPayload delete_answer_sheet = <next_tag>;
```

#### Extend InsertData oneof:

```protobuf
SchemePageInsert scheme_page = 36;   // next after reserved tag 35
AnswerPageInsert answer_page = 37;
```

#### New SyncAction values (if the server mirrors the enum):

```
uploadScheme = 91
deleteScheme = 92
uploadAnswerSheet = 93
deleteAnswerSheet = 94
```

#### Generate code:

Run `protoc` (or `buf generate`, or whatever the project uses) to produce updated Rust stubs.

**Update after completion:**
- [x] Add all 6 new message types to sync.proto
- [x] Extend ActionRequest oneof with 4 new payload variants
- [x] Extend InsertData oneof with tags 36 and 37
- [x] Generate Rust code — `cargo build` succeeds
- [x] **NOTIFY USER:** "S1 complete — client can regenerate proto stubs."
- [x] Mark this task `[x]`
- [ ] git commit: `feat: add proto definitions for scheme/answer file sync (4 actions, 2 insert types)`

---

### Task S2: Create `scheme_pages` and `answer_pages` tables in schema

**Files to modify:** Schema/migration files (e.g., `migrations/`, `schema.sql`, or equivalent)
**Depends on:** S1
**Parallel group:** P2 (sequential with S3–S5)

**Specification:**

```sql
CREATE TABLE scheme_pages (
  school    TEXT     NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
  exam      TEXT     NOT NULL,
  subject   INTEGER  NOT NULL,
  paper     INTEGER,          -- nullable: NULL = single-paper subject
  page      SMALLINT NOT NULL, -- 0-indexed page number
  key       TEXT     NOT NULL, -- S3 object key
  created   BIGINT   NOT NULL, -- seconds since epoch
  PRIMARY KEY (school, exam, subject, paper, page),
  FOREIGN KEY (school, exam) REFERENCES exams(school, id) ON DELETE CASCADE,
  FOREIGN KEY (subject) REFERENCES subjects(id) ON DELETE CASCADE
);

CREATE TABLE answer_pages (
  school    TEXT     NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
  exam      TEXT     NOT NULL,
  student   INTEGER  NOT NULL,
  subject   INTEGER  NOT NULL,
  paper     INTEGER,          -- nullable
  page      SMALLINT NOT NULL,
  key       TEXT     NOT NULL,
  created   BIGINT   NOT NULL,
  PRIMARY KEY (school, exam, student, subject, paper, page),
  FOREIGN KEY (school, exam) REFERENCES exams(school, id) ON DELETE CASCADE,
  FOREIGN KEY (school, student) REFERENCES students(school, adm) ON DELETE CASCADE,
  FOREIGN KEY (subject) REFERENCES subjects(id) ON DELETE CASCADE
);

-- Performance indexes
CREATE INDEX idx_scheme_pages_paper ON scheme_pages(school, exam, subject, paper);
CREATE INDEX idx_answer_pages_student ON answer_pages(school, exam, student, subject, paper);
```

**Note on NULL paper in Postgres PK:** Postgres handles NULL in composite PKs differently than SQLite. If the server uses Postgres, NULL values in PKs are considered distinct (each NULL is unique). If this causes issues, consider adding a `COALESCE(paper, -1)` expression index or using -1 as a sentinel. Check the server's Postgres version and behavior.

**Update after completion:**
- [ ] Create `scheme_pages` table with indexes
- [ ] Create `answer_pages` table with indexes
- [ ] Run migration — database accepts the new tables
- [ ] Mark this task `[x]`
- [ ] git commit: `db: add scheme_pages and answer_pages tables for file sync metadata`

---

### Task S3: Implement uploadScheme and deleteScheme action handlers

**Files to modify:** Action handler dispatch (e.g., `src/services/actions.rs` or equivalent), S3 presigned URL generation
**Depends on:** S2
**Parallel group:** P2

**Specification:**

#### `uploadScheme` handler:

Input: `UploadSchemePayload { school, exam, subject, paper?, count }`

1. **Permission check:** User must have write access to the paper (member of school with appropriate role).

2. **Delete existing scheme pages:**
   ```sql
   DELETE FROM scheme_pages
   WHERE school = $1 AND exam = $2 AND subject = $3
     AND paper IS NOT DISTINCT FROM $4;
   ```
   For each deleted row, append a DELETE changelog entry.

3. **Create new rows + S3 keys:**
   For `page` in `0..count-1`:
   ```sql
   INSERT INTO scheme_pages (school, exam, subject, paper, page, key, created)
   VALUES ($1, $2, $3, $4, $page, $key, $now);
   ```
   S3 key format: `schemes/{school}/{exam}/{subject}_{paper}/{page}.jpg`
   (Use `0` for paper when NULL.)

   For each inserted row, append an INSERT changelog entry with table=36.

4. **Generate presigned PUT URLs** for each new S3 key.

5. **Build ActionResponse:**
   - `success = true`
   - `rows`: one `ActionRow` per inserted row (table=36, operation=0, rowKey=`{school}|{exam}|{subject}|{paper}|{page}`, data=`SchemePageInsert`)
   - `file_urls`: one `FileUrl` per page:
     ```
     FileUrl {
       path: "submissions/{school}/{exam}/{subject}_{paper}/scheme/{page}.jpg",
       put_url: "<presigned PUT URL>",
       get_url: "",  // empty for push originator
       expiry: 0,
     }
     ```

#### `deleteScheme` handler:

Input: `DeleteSchemePayload { school, exam, subject, paper? }`

1. **Permission check.**
2. **Load existing rows** (to get S3 keys for deletion).
3. **Delete all rows:**
   ```sql
   DELETE FROM scheme_pages
   WHERE school = $1 AND exam = $2 AND subject = $3
     AND paper IS NOT DISTINCT FROM $4;
   ```
4. **Delete S3 objects** (fire-and-forget or background task).
5. **Append DELETE changelog entries** for each deleted row (table=36, operation=2).
6. **Return success** (no file_urls needed for deletes).

#### Watch stream deltas for other clients:

When broadcasting to watchers, each changelog entry for scheme_pages should include:
- For INSERTs: `SyncDelta { table=36, operation=0, rowKey="...", data=SchemePageInsert{...}, file_urls=[FileUrl{path: "submissions/...", get_url: "<presigned GET URL>", expiry: <1 month>}] }`
- For DELETEs: `SyncDelta { table=36, operation=2, rowKey="...", file_urls=[] }`

**Update after completion:**
- [ ] Implement `uploadScheme` handler with S3 PUT URL generation
- [ ] Implement `deleteScheme` handler with S3 object deletion
- [ ] Wire handlers into action dispatch
- [ ] Changelog entries produce correct table=36 deltas
- [ ] `cargo build` succeeds
- [ ] Test: upload scheme → verify scheme_pages rows + PUT URLs returned
- [ ] Mark this task `[x]`
- [ ] git commit: `feat: implement uploadScheme and deleteScheme action handlers with S3 integration`

---

### Task S4: Implement uploadAnswerSheet and deleteAnswerSheet action handlers

**Files to modify:** Same as S3
**Depends on:** S3 (same pattern, can reuse S3 URL generation code)
**Parallel group:** P2

**Specification:**

Identical pattern to S3, but for `answer_pages` table (table=37).

#### `uploadAnswerSheet` handler:

Input: `UploadAnswerSheetPayload { school, exam, student, subject, paper?, count }`

1. **Permission check:** User must have access to this student's data.
2. **Delete existing answer pages:**
   ```sql
   DELETE FROM answer_pages
   WHERE school = $1 AND exam = $2 AND student = $3 AND subject = $4
     AND paper IS NOT DISTINCT FROM $5;
   ```
3. **Create new rows:**
   S3 key format: `answers/{school}/{exam}/{subject}_{paper}/{student}/{page}.jpg`
   rowKey format: `{school}|{exam}|{student}|{subject}|{paper}|{page}`
4. **Generate presigned PUT URLs.**
5. **Build ActionResponse** with table=37 ActionRows and FileUrl entries:
   ```
   FileUrl {
     path: "submissions/{school}/{exam}/{subject}_{paper}/{student}/{page}.jpg",
     put_url: "<presigned PUT URL>",
   }
   ```

#### `deleteAnswerSheet` handler:

Same pattern as `deleteScheme` but for `answer_pages` (table=37).

#### Watch stream deltas:

Same pattern as S3 but with table=37 and `AnswerPageInsert` data.

**Update after completion:**
- [ ] Implement `uploadAnswerSheet` handler
- [ ] Implement `deleteAnswerSheet` handler
- [ ] Wire handlers into action dispatch
- [ ] Changelog entries produce correct table=37 deltas
- [ ] `cargo build` succeeds
- [ ] Test: upload answer sheet → verify answer_pages rows + PUT URLs returned
- [ ] Mark this task `[x]`
- [ ] git commit: `feat: implement uploadAnswerSheet and deleteAnswerSheet action handlers`

---

### Task S5: Wire new tables into watch stream delta builder

**Files to modify:** Watch stream / changelog reader code (e.g., `src/services/sync.rs` or equivalent delta builder)
**Depends on:** S3, S4
**Parallel group:** P2

**Specification:**

Ensure the `watchChanges` stream correctly builds `SyncDelta` messages for `scheme_pages` (table=36) and `answer_pages` (table=37) changelog entries.

For each changelog row with table=36 or table=37:

1. **For INSERT/UPDATE operations:**
   - Read the row from the relevant table using the rowKey.
   - Build the appropriate `InsertData` (`SchemePageInsert` or `AnswerPageInsert`).
   - Generate presigned GET URLs for each page's S3 key.
   - Attach `FileUrl { path: "submissions/...", get_url: "<GET URL>", expiry: <1 month> }`.

2. **For DELETE operations:**
   - Just send the `SyncDelta` with `operation=2` and `rowKey` (no data needed).
   - The client delta writer will delete the local row and optionally the local file.

**S3 GET URL expiry:** Use ~1 month (same as profile image URLs per AGENT.md §8).

**Row key formats:**
- scheme_pages: `{school}|{exam}|{subject}|{paper}|{page}` (paper is empty string when NULL)
- answer_pages: `{school}|{exam}|{student}|{subject}|{paper}|{page}` (paper is empty string when NULL)

**Update after completion:**
- [ ] Watch stream handles table=36 (scheme_pages) changelog entries
- [ ] Watch stream handles table=37 (answer_pages) changelog entries
- [ ] GET URLs generated with ~1 month expiry
- [ ] `cargo build` succeeds
- [ ] End-to-end test: Device A uploads scheme → Device B receives delta with GET URL → downloads file
- [ ] Mark this task `[x]`
- [ ] git commit: `feat: wire scheme_pages and answer_pages into watch stream delta builder`
