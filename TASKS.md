# Ledger — Task Board

> Tasks for the Exam/Paper Schema Redesign.
> Remove `exam_grades` table, add `grade`/`stream` columns to `papers`.

---

## Context: What Changed and Why

The current `exam_grades` junction table (PK: exam, grade, stream) is **useless** because:
1. Papers have no `grade` or `stream` columns — there's no way to link a paper to a specific grade/stream
2. The exam list UI doesn't display exam names properly
3. When viewing an exam, no papers/grades/streams appear even though `papers` rows exist in the DB

### The Fix

**Remove** the `exam_grades` table entirely and **add two columns to `papers`**:
- `grade SMALLINT NOT NULL` — which grade this paper is for
- `stream SMALLINT` — nullable; NULL means all streams

This makes papers self-describing — each paper knows which class it belongs to.

---

### Task L01: Create Diesel migration — add grade/stream to papers, drop exam_grades [x]

**Files to create:** `migrations/2026-03-14-000000-papers_grade_stream/up.sql`, `migrations/2026-03-14-000000-papers_grade_stream/down.sql`
**Depends on:** None
**Parallel group:** P1

**Specification:**

Create a new Diesel migration directory `migrations/2026-03-14-000000-papers_grade_stream/`.

**`up.sql`:**
```sql
-- Step 1: Add new columns to papers
ALTER TABLE papers ADD COLUMN grade SMALLINT NOT NULL DEFAULT 0;
ALTER TABLE papers ADD COLUMN stream SMALLINT;

-- Step 2: Backfill from exam_grades (pick first matching exam_grade row per paper)
UPDATE papers SET
    grade = (SELECT eg.grade FROM exam_grades eg WHERE eg.exam = papers.exam LIMIT 1),
    stream = (SELECT eg.stream FROM exam_grades eg WHERE eg.exam = papers.exam LIMIT 1)
WHERE EXISTS (SELECT 1 FROM exam_grades eg WHERE eg.exam = papers.exam);

-- Step 3: Drop exam_grades table
DROP TABLE IF EXISTS exam_grades;
DROP INDEX IF EXISTS idx_exam_grades_grade;

-- Step 4: Recreate triggers that referenced exam_grades
-- They now JOIN through papers instead

DROP TRIGGER IF EXISTS grades_enrollment_check;
CREATE TRIGGER grades_enrollment_check
    BEFORE INSERT ON grades
BEGIN
    SELECT RAISE(ABORT, 'student is not enrolled in any class this exam covers')
    WHERE NOT EXISTS (
        SELECT 1 FROM enrollments
        INNER JOIN exams ON exams.id = NEW.exam AND exams.school = NEW.school
        INNER JOIN papers ON papers.exam = exams.id AND papers.school = exams.school
                         AND papers.subject = NEW.subject
                         AND (papers.paper = NEW.paper OR (papers.paper IS NULL AND NEW.paper IS NULL))
        WHERE enrollments.school  = NEW.school
          AND enrollments.student = NEW.student
          AND enrollments.year    = exams.year
          AND enrollments.term    = exams.term
          AND enrollments.grade   = papers.grade
          AND (papers.stream IS NULL OR enrollments.stream = papers.stream)
    );
END;

DROP TRIGGER IF EXISTS grades_enrollment_check_update;
CREATE TRIGGER grades_enrollment_check_update
    BEFORE UPDATE ON grades
BEGIN
    SELECT RAISE(ABORT, 'student is not enrolled in any class this exam covers')
    WHERE NOT EXISTS (
        SELECT 1 FROM enrollments
        INNER JOIN exams ON exams.id = NEW.exam AND exams.school = NEW.school
        INNER JOIN papers ON papers.exam = exams.id AND papers.school = exams.school
                         AND papers.subject = NEW.subject
                         AND (papers.paper = NEW.paper OR (papers.paper IS NULL AND NEW.paper IS NULL))
        WHERE enrollments.school  = NEW.school
          AND enrollments.student = NEW.student
          AND enrollments.year    = exams.year
          AND enrollments.term    = exams.term
          AND enrollments.grade   = papers.grade
          AND (papers.stream IS NULL OR enrollments.stream = papers.stream)
    );
END;
```

**`down.sql`:**
```sql
-- Recreate exam_grades table
CREATE TABLE exam_grades (
    exam    TEXT     NOT NULL REFERENCES exams(id) ON DELETE CASCADE,
    grade   SMALLINT NOT NULL,
    stream  SMALLINT NOT NULL,
    PRIMARY KEY (exam, grade, stream)
);
CREATE INDEX idx_exam_grades_grade ON exam_grades(grade, stream);

-- Backfill exam_grades from papers
INSERT OR IGNORE INTO exam_grades (exam, grade, stream)
SELECT DISTINCT exam, grade, COALESCE(stream, 0) FROM papers WHERE grade IS NOT NULL;

-- Remove columns from papers (SQLite requires table rebuild)
CREATE TABLE papers_backup AS SELECT school, exam, subject, paper, topic, invigilator, start, "end", status, created, updated FROM papers;
DROP TABLE papers;
CREATE TABLE papers (
    school text not null,
    exam text not null,
    subject integer not null,
    paper smallint,
    topic integer,
    invigilator text not null,
    start bigint not null,
    "end" bigint not null,
    status smallint not null default 0,
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    CHECK (start < "end"),
    primary key (school, exam, subject, paper),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (exam) references exams(id) ON DELETE CASCADE,
    foreign key (subject) references subjects(id) ON DELETE CASCADE,
    foreign key (topic) references topics(id) ON DELETE SET NULL,
    foreign key (school, invigilator) references teachers(school, user) ON DELETE CASCADE
);
INSERT INTO papers SELECT * FROM papers_backup;
DROP TABLE papers_backup;

-- Recreate original triggers
DROP TRIGGER IF EXISTS grades_enrollment_check;
CREATE TRIGGER grades_enrollment_check
    BEFORE INSERT ON grades
BEGIN
    SELECT RAISE(ABORT, 'student is not enrolled in any class this exam covers')
    WHERE NOT EXISTS (
        SELECT 1 FROM enrollments
        INNER JOIN exams ON exams.id = NEW.exam AND exams.school = NEW.school
        INNER JOIN exam_grades ON exam_grades.exam = exams.id
        WHERE enrollments.school  = NEW.school
          AND enrollments.student = NEW.student
          AND enrollments.year    = exams.year
          AND enrollments.term    = exams.term
          AND enrollments.grade   = exam_grades.grade
          AND enrollments.stream  = exam_grades.stream
    );
END;

DROP TRIGGER IF EXISTS grades_enrollment_check_update;
CREATE TRIGGER grades_enrollment_check_update
    BEFORE UPDATE ON grades
BEGIN
    SELECT RAISE(ABORT, 'student is not enrolled in any class this exam covers')
    WHERE NOT EXISTS (
        SELECT 1 FROM enrollments
        INNER JOIN exams ON exams.id = NEW.exam AND exams.school = NEW.school
        INNER JOIN exam_grades ON exam_grades.exam = exams.id
        WHERE enrollments.school  = NEW.school
          AND enrollments.student = NEW.student
          AND enrollments.year    = exams.year
          AND enrollments.term    = exams.term
          AND enrollments.grade   = exam_grades.grade
          AND enrollments.stream  = exam_grades.stream
    );
END;
```

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task L02: Regenerate Diesel schema [x]

**Files to modify:** `src/db/schema/schema.rs`
**Depends on:** Task L01
**Parallel group:** —

**Specification:**

Run:
```sh
diesel migration run
diesel print-schema > src/db/schema/schema.rs
```

Verify:
- `papers` table now has `grade -> SmallInt` and `stream -> Nullable<SmallInt>`
- `exam_grades` table is gone
- All `joinable!` macros referencing `exam_grades` are removed

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task L03: Update rows.rs — remove ExamGradeRow, add grade/stream to PaperRow [x]

**Files to modify:** `src/db/database/tables/rows.rs`
**Depends on:** Task L02
**Parallel group:** P2

**Specification:**

1. **Remove** `ExamGradeRow` struct entirely and its `impl` block (row_key, school_id, From<&Row>).

2. **Update `PaperRow`** — add two fields:
   ```rust
   pub grade: i16,
   pub stream: Option<i16>,
   ```

3. Update `PaperRow`'s `From<&Row>` impl to read `grade` and `stream` from the DB row.

4. Update `PaperRow`'s `row_key()` — this shouldn't change since paper PK is still `(school, exam, subject, paper)`.

5. Remove any `From<&Row> for ExamGradeInsert` implementation.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task L04: Update insert.rs — remove insert_exam_grade, update insert_paper [x]

**Files to modify:** `src/db/database/tables/insert.rs`
**Depends on:** Task L02
**Parallel group:** P2

**Specification:**

1. **Remove** `insert_exam_grade` function entirely.

2. **Update `insert_paper`** to include `grade` and `stream` columns:
   - Add `.value(papers::grade, &row.grade)` to the insert statement
   - Add `.value(papers::stream, &row.stream)` to the insert statement

3. Remove any imports related to `exam_grades` schema.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task L05: Update delete.rs — remove delete_exam_grade [x]

**Files to modify:** `src/db/database/tables/delete.rs`
**Depends on:** Task L02
**Parallel group:** P2

**Specification:**

1. **Remove** `delete_exam_grade` function if it exists.
2. Remove any imports related to `exam_grades` schema.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task L06: Update snapshot.rs — remove exam_grades snapshot, update papers [x]

**Files to modify:** `src/db/database/tables/snapshot.rs`
**Depends on:** Task L02
**Parallel group:** P2

**Specification:**

1. **Remove** the `exam_grades` match arm from `snapshot_table()` and `snapshot_table_since()`.

2. **Update** the `papers` match arm in both functions to include `grade` and `stream` in the SELECT and in the `PaperInsert` construction.

3. Remove any `use` imports for `exam_grades`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task L07: Update actions.rs — remove exam_grade handlers, update paper handlers [x]

**Files to modify:** `src/db/database/tables/actions.rs`
**Depends on:** Tasks L03, L04, L05
**Parallel group:** —

**Specification:**

1. **Remove** `handle_add_exam_grade` function entirely.
2. **Remove** `handle_remove_exam_grade` function entirely.
3. **Remove** `TBL_EXAM_GRADES` constant (or mark as deprecated/unused).
4. **Remove** `fetch_exam_grade` helper if it exists.

5. **Update `execute_action` dispatcher:**
   - Remove match arms for sync_action values 89 and 90.
   - Add comments: `// 89, 90: reserved (removed exam_grade actions)`

6. **Update `action_permission`:**
   - Remove entries for actions 89 and 90.

7. **Update `handle_create_paper`:**
   - Read `grade` from `CreatePaperPayload` field 9 (int32).
   - Read `stream` from `CreatePaperPayload` field 10 (optional int32).
   - Include these in the PaperRow / insert call.

8. **Update `handle_create_exam`:**
   - Remove the loop that inserts `exam_grades` rows from `CreateExamPayload.grades`.
   - The `CreateExamPayload` will no longer have a `grades` field.

9. Remove any references to `insert_exam_grade` or `delete_exam_grade`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task L08: Update services/sync.rs — remove ExamGrades from LogTable [x]

**Files to modify:** `src/services/sync.rs`
**Depends on:** Task L02
**Parallel group:** P2

**Specification:**

1. **Remove** `ExamGrades` variant from the `LogTable` enum.
2. **Remove** `ExamGrades` from `SNAPSHOT_TABLE_ORDER` array.
3. **Remove** `ExamGrades` from `LogTable::from_i32()` match.
4. **Remove** `ExamGrades` from `LogTable::resource()` match.
5. **Remove** `ExamGrades` from `LogTable::school_from_key()` match.
6. **Remove** `ExamGrades` from `SyncFilter` if referenced.
7. Update table count comment if present (34 synced tables → 33).

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task L09: Update sync.proto — add grade/stream to paper messages, remove exam_grade messages [ ]

**Files to modify:** `protos/services/sync.proto`
**Depends on:** None
**Parallel group:** P1

**Specification:**

1. **Update `PaperInsert`** — add two fields after status:
   ```protobuf
   message PaperInsert {
     string school = 1;
     string exam = 2;
     int32 subject = 3;
     optional int32 paper = 4;
     optional int32 topic = 5;
     string invigilator = 6;
     int64 start = 7;
     int64 end = 8;
     int32 status = 9;
     int32 grade = 10;
     optional int32 stream = 11;
   }
   ```

2. **Update `CreatePaperPayload`** — add grade/stream:
   ```protobuf
   message CreatePaperPayload {
     string school = 1;
     string exam = 2;
     int32 subject = 3;
     optional int32 paper = 4;
     string invigilator = 5;
     int64 start = 6;
     int64 end = 7;
     optional int32 topic = 8;
     int32 grade = 9;
     optional int32 stream = 10;
   }
   ```

3. **Update `UpdatePaperPayload`** — add optional grade/stream:
   ```protobuf
   message UpdatePaperPayload {
     string school = 1;
     string exam = 2;
     int32 subject = 3;
     optional int32 paper = 4;
     optional string invigilator = 5;
     optional int64 start = 6;
     optional int64 end = 7;
     optional int32 status = 8;
     optional int32 topic = 9;
     optional int32 grade = 10;
     optional int32 stream = 11;
   }
   ```

4. **Remove** `ExamGradeEntry` message entirely.

5. **Update `CreateExamPayload`** — remove `repeated ExamGradeEntry grades = 11;`:
   ```protobuf
   message CreateExamPayload {
     string id = 1;
     string school = 2;
     string name = 3;
     int32 year = 4;
     int32 term = 5;
     bool personalized = 6;
     int32 type = 7;
     int32 start = 8;
     int32 end = 9;
     string teacher = 10;
     // Field 11 removed (was: repeated ExamGradeEntry grades)
   }
   ```

6. **Remove** `AddExamGradePayload` message entirely.

7. **Remove** `RemoveExamGradePayload` message entirely.

8. **Remove** `ExamGradeInsert` message entirely.

9. **Update `InsertData` oneof** — remove exam_grade, mark reserved:
   ```protobuf
   // ExamGradeInsert exam_grade = 35;  // REMOVED — field number reserved
   reserved 35;
   ```

10. Add comment near sync_action values 89/90 documentation (if any) noting they are reserved.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task L10: Also update the base migration SQL to match [x]

**Files to modify:** `migrations/2026-02-21-013710-0000_tables/up.sql`
**Depends on:** Task L09
**Parallel group:** —

**Specification:**

Update the base migration to reflect the final schema state (for fresh installs):

1. **Remove** the `CREATE TABLE exam_grades` block and its index.

2. **Update `CREATE TABLE papers`** — add `grade` and `stream` columns:
   ```sql
   CREATE TABLE papers (
       school text not null,
       exam text not null,
       subject integer not null,
       paper smallint,
       topic integer,
       invigilator text not null,
       start bigint not null,
       "end" bigint not null,
       status smallint not null default 0,
       grade smallint not null,
       stream smallint,
       created bigint not null default (unixepoch('now')),
       updated bigint not null default (unixepoch('now')),
       CHECK (start < "end"),
       primary key (school, exam, subject, paper),
       foreign key (school) references schools(id) ON DELETE CASCADE,
       foreign key (exam) references exams(id) ON DELETE CASCADE,
       foreign key (subject) references subjects(id) ON DELETE CASCADE,
       foreign key (topic) references topics(id) ON DELETE SET NULL,
       foreign key (school, invigilator) references teachers(school, user) ON DELETE CASCADE
   );
   ```

3. **Update triggers** `grades_enrollment_check` and `grades_enrollment_check_update` to JOIN through `papers` instead of `exam_grades` (same SQL as in Task L01 up.sql).

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task L11: Build and verify [ ]

**Files to modify:** None (verification only)
**Depends on:** All previous tasks
**Parallel group:** —

**Specification:**

1. Delete the existing `database.db` file.
2. Run `diesel migration run` to recreate from the updated base migration + new migration.
3. Run `diesel print-schema > src/db/schema/schema.rs` to ensure schema is in sync.
4. Run `cargo build` — fix ALL compilation errors.
5. Run `cargo test` — fix any test failures.

Common errors to expect:
- Missing imports for removed `exam_grades` schema module
- Type mismatches from new `grade`/`stream` fields on PaperRow
- Dead code warnings from removed functions
- Unused import warnings

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

## Dependency Graph

```
L01 (migration) ──→ L02 (schema regen) ──→ L03 (rows.rs)    ──┐
                                         ──→ L04 (insert.rs)  ──├──→ L07 (actions.rs) ──→ L11 (verify)
                                         ──→ L05 (delete.rs)  ──┘         ↑
                                         ──→ L06 (snapshot.rs) ────────────┘
                                         ──→ L08 (sync.rs)     ────────────┘
L09 (sync.proto) ──→ L10 (base migration update) ──→ L11 (verify)
```

**Parallel groups:**
- P1: L01, L09 (no dependencies between them)
- P2: L03, L04, L05, L06, L08 (all depend on L02, independent of each other)
