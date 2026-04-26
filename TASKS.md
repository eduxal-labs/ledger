# TASKS.md — Paper/Exam Redesign + Question Schema Enhancement

## Status Legend
- [ ] Pending
- [x] Complete

---

## Dependency Graph (read before executing)

```
A1 → A2 → B1..B7 (parallel) → B8, B9 (after B1-B7)
A2 → C1..C5 (parallel, after B tasks)
A2 → D1..D6 (parallel, after B tasks)
D1..D6 → E1..E5 (parallel)
C1..C5 + E1..E5 → F1..F6 (parallel)
B1..B2 → G1 → G2 → G3
F1..F6 + G1..G3 → H1
```

---

## Track A: Database Schema Migration

### Task A1: Clean-slate migration SQL
**Files to create:** `migrations/2026-06-01-000000-0007_paper_event_redesign/up.sql`, `migrations/2026-06-01-000000-0007_paper_event_redesign/down.sql`
**Reference files:** `migrations/2026-02-21-013710-0000_tables/up.sql`, `migrations/2026-03-30-000000-0002_question_bank/up.sql`
**Depends on:** nothing
**Parallel group:** sequential

**Specification:**

Create the directory and write the migration. The `up.sql` must:

1. DROP old tables in reverse-dependency order (SQLite CASCADE does not auto-drop dependents on DROP TABLE, so explicit ordering is required):

```sql
-- ============================================================
-- 0007 up: Paper/event redesign + question schema clean slate
-- ============================================================

-- Drop in reverse dependency order
DROP TABLE IF EXISTS marking_queue;
DROP TABLE IF EXISTS question_grades;
DROP TABLE IF EXISTS paper_questions;
DROP TABLE IF EXISTS grades;
DROP TABLE IF EXISTS answer_pages;
DROP TABLE IF EXISTS scheme_pages;
DROP TABLE IF EXISTS papers;
DROP TABLE IF EXISTS exams;
DROP TABLE IF EXISTS question_images;
DROP TABLE IF EXISTS rubric_criteria;
DROP TABLE IF EXISTS questions;
```

2. Recreate question-bank tables with new schema:

```sql
-- ============================================================
-- Question bank (clean slate)
-- ============================================================

-- body_format: 0=plain, 1=tiptap
-- type_:       0=definition, 1=explanation, 2=calculation,
--              3=structured, 4=experiment, 5=data_response, 6=diagram
-- difficulty:  1..5
-- cognitive_level: 0=recall, 1=comprehension, 2=application, 3=analysis
-- answer_space_type: 0=lines, 1=plain_box, 2=diagram_box,
--                    3=construction_box, 4=grid_box
-- stimulus:    JSON { type:int, body:str, body_format:int,
--                     caption:str, image:{filename,caption,description}|null }
-- example_answer: JSON { format:int, content:str|null,
--                         image:{filename,caption,description}|null }
--   format: 0=plain, 1=tiptap, 2=svg, 3=image
-- max_marks: caps how many rubric criteria can be awarded (nullable = no cap)

CREATE TABLE questions (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    topic               INTEGER  NOT NULL,
    body                TEXT     NOT NULL,
    body_format         SMALLINT NOT NULL DEFAULT 0,
    stimulus            TEXT,
    type_               SMALLINT NOT NULL DEFAULT 0,
    difficulty          SMALLINT NOT NULL DEFAULT 3 CHECK (difficulty BETWEEN 1 AND 5),
    cognitive_level     SMALLINT NOT NULL DEFAULT 0,
    marks               SMALLINT NOT NULL,
    max_marks           SMALLINT,
    answer_space_type   SMALLINT NOT NULL DEFAULT 0,
    answer_lines        SMALLINT,
    answer_box_height_mm SMALLINT,
    example_answer      TEXT,
    created             BIGINT   NOT NULL DEFAULT (unixepoch('now')),
    updated             BIGINT   NOT NULL DEFAULT (unixepoch('now')),
    created_by          TEXT     NOT NULL,
    FOREIGN KEY (topic)      REFERENCES topics(id)   ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES users(id)    ON DELETE CASCADE
);
CREATE INDEX idx_questions_topic ON questions(topic);
CREATE INDEX idx_questions_topic_marks ON questions(topic, marks);
CREATE UNIQUE INDEX idx_questions_topic_body ON questions(topic, body);

-- rubric_criteria: atomic — one point per criterion
-- max_marks: caps how many criteria can be awarded for this question
-- required: if TRUE, this criterion must be awarded (no substitution)
CREATE TABLE rubric_criteria (
    question  INTEGER  NOT NULL,
    position  SMALLINT NOT NULL,
    criterion TEXT     NOT NULL,
    marks     SMALLINT NOT NULL,
    max_marks SMALLINT,
    required  BOOLEAN  NOT NULL DEFAULT FALSE,
    PRIMARY KEY (question, position),
    FOREIGN KEY (question) REFERENCES questions(id) ON DELETE CASCADE
);

-- Parts are sub-questions with their own body, marks, rubric, answer space, etc.
CREATE TABLE question_parts (
    question             INTEGER  NOT NULL,
    position             SMALLINT NOT NULL,
    label                TEXT     NOT NULL,
    body                 TEXT     NOT NULL,
    body_format          SMALLINT NOT NULL DEFAULT 0,
    marks                SMALLINT NOT NULL,
    max_marks            SMALLINT,
    answer_space_type    SMALLINT NOT NULL DEFAULT 0,
    answer_lines         SMALLINT,
    answer_box_height_mm SMALLINT,
    example_answer       TEXT,
    stimulus             TEXT,
    PRIMARY KEY (question, position),
    FOREIGN KEY (question) REFERENCES questions(id) ON DELETE CASCADE
);
CREATE INDEX idx_question_parts_question ON question_parts(question);

CREATE TABLE part_rubric_criteria (
    question  INTEGER  NOT NULL,
    part      SMALLINT NOT NULL,
    position  SMALLINT NOT NULL,
    criterion TEXT     NOT NULL,
    marks     SMALLINT NOT NULL,
    max_marks SMALLINT,
    required  BOOLEAN  NOT NULL DEFAULT FALSE,
    PRIMARY KEY (question, part, position),
    FOREIGN KEY (question, part)
        REFERENCES question_parts(question, position) ON DELETE CASCADE
);

CREATE TABLE question_images (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    question INTEGER  NOT NULL,
    position SMALLINT NOT NULL,
    context  SMALLINT NOT NULL,
    key      TEXT     NOT NULL,
    caption  TEXT,
    FOREIGN KEY (question) REFERENCES questions(id) ON DELETE CASCADE
);
CREATE INDEX idx_question_images_question ON question_images(question);
```

3. New structural tables:

```sql
-- ============================================================
-- Events (replaces exams)
-- type: 0=exam, 1=mock, 2=holiday_revision
-- status: 0=draft, 1=active, 2=completed, 3=cancelled
-- ============================================================
CREATE TABLE events (
    id         TEXT     PRIMARY KEY NOT NULL,
    school     TEXT     NOT NULL,
    name       TEXT     NOT NULL,
    type_      SMALLINT NOT NULL DEFAULT 0,
    term       SMALLINT NOT NULL,
    year       INTEGER  NOT NULL,
    start_date INTEGER  NOT NULL,
    end_date   INTEGER  NOT NULL,
    status     SMALLINT NOT NULL DEFAULT 0,
    created    BIGINT   NOT NULL DEFAULT (unixepoch('now')),
    updated    BIGINT   NOT NULL DEFAULT (unixepoch('now')),
    CHECK (start_date <= end_date),
    FOREIGN KEY (school) REFERENCES schools(id) ON DELETE CASCADE
);
CREATE INDEX idx_events_school ON events(school, year, term);

-- ============================================================
-- Papers (atomic unit — replaces old composite papers + exams)
-- type:            0=exam, 1=cat, 2=assessment, 3=assignment,
--                  4=practical, 5=adaptive
-- status:          0=draft, 1=questions_set, 2=finalized, 3=revealed,
--                  4=active, 5=completed, 6=marked
-- generation_mode: 0=class_uniform, 1=per_student
-- ============================================================
CREATE TABLE papers (
    id              TEXT     PRIMARY KEY NOT NULL,
    school          TEXT     NOT NULL,
    event           TEXT,
    subject         INTEGER  NOT NULL,
    grade           SMALLINT NOT NULL,
    stream          SMALLINT,
    type_           SMALLINT NOT NULL DEFAULT 0,
    teacher         TEXT     NOT NULL,
    name            TEXT     NOT NULL,
    total_marks     SMALLINT NOT NULL,
    duration_minutes SMALLINT NOT NULL,
    date            INTEGER  NOT NULL,
    status          SMALLINT NOT NULL DEFAULT 0,
    pdf_key         TEXT,
    ms_key          TEXT,
    generation_mode SMALLINT NOT NULL DEFAULT 0,
    instructions    TEXT,
    created         BIGINT   NOT NULL DEFAULT (unixepoch('now')),
    updated         BIGINT   NOT NULL DEFAULT (unixepoch('now')),
    FOREIGN KEY (school)          REFERENCES schools(id)   ON DELETE CASCADE,
    FOREIGN KEY (event)           REFERENCES events(id)    ON DELETE SET NULL,
    FOREIGN KEY (subject)         REFERENCES subjects(id)  ON DELETE CASCADE,
    FOREIGN KEY (school, teacher) REFERENCES teachers(school, user) ON DELETE CASCADE
);
CREATE INDEX idx_papers_school ON papers(school, grade, subject);
CREATE INDEX idx_papers_event  ON papers(event);

-- ============================================================
-- Paper schedules (one row per paper slot within an event)
-- generation_status: 0=pending, 1=generating, 2=generated, 3=failed
-- ============================================================
CREATE TABLE paper_schedules (
    id                TEXT     PRIMARY KEY NOT NULL,
    event             TEXT     NOT NULL,
    subject           INTEGER  NOT NULL,
    grade             SMALLINT NOT NULL,
    stream            SMALLINT,
    date              INTEGER  NOT NULL,
    start_time        INTEGER  NOT NULL,
    end_time          INTEGER  NOT NULL,
    duration_minutes  SMALLINT NOT NULL,
    invigilator       TEXT,
    paper             TEXT,
    generation_status SMALLINT NOT NULL DEFAULT 0,
    reveal_at         BIGINT   NOT NULL,
    generate_at       BIGINT   NOT NULL,
    created           BIGINT   NOT NULL DEFAULT (unixepoch('now')),
    CHECK (start_time < end_time),
    FOREIGN KEY (event)       REFERENCES events(id)   ON DELETE CASCADE,
    FOREIGN KEY (subject)     REFERENCES subjects(id) ON DELETE CASCADE,
    FOREIGN KEY (invigilator) REFERENCES users(id)    ON DELETE SET NULL,
    FOREIGN KEY (paper)       REFERENCES papers(id)   ON DELETE SET NULL
);
CREATE INDEX idx_paper_schedules_event   ON paper_schedules(event);
CREATE INDEX idx_paper_schedules_pending ON paper_schedules(generation_status, generate_at)
    WHERE generation_status = 0;

-- ============================================================
-- Taught topics — per-school tracking of curriculum coverage
-- status: 0=not_started, 1=in_progress, 2=completed
-- stream NULL means applies to all streams in the grade
-- ============================================================
CREATE TABLE taught_topics (
    school      TEXT     NOT NULL,
    subject     INTEGER  NOT NULL,
    grade       SMALLINT NOT NULL,
    stream      SMALLINT,
    topic       INTEGER  NOT NULL,
    taught_by   TEXT     NOT NULL,
    status      SMALLINT NOT NULL DEFAULT 0,
    taught_date INTEGER,
    updated     BIGINT   NOT NULL DEFAULT (unixepoch('now')),
    PRIMARY KEY (school, subject, grade, stream, topic),
    FOREIGN KEY (school)              REFERENCES schools(id)  ON DELETE CASCADE,
    FOREIGN KEY (subject)             REFERENCES subjects(id) ON DELETE CASCADE,
    FOREIGN KEY (topic)               REFERENCES topics(id)   ON DELETE CASCADE,
    FOREIGN KEY (school, taught_by)   REFERENCES teachers(school, user) ON DELETE CASCADE
);
CREATE INDEX idx_taught_topics_school_subject ON taught_topics(school, subject, grade);

-- ============================================================
-- Exam coverage — admin-confirmed point-in-time snapshot of
-- taught topics for a specific paper_schedule. This is what
-- the generation algorithm reads (NOT live taught_topics).
-- ============================================================
CREATE TABLE exam_coverage (
    schedule      TEXT    NOT NULL,
    topic         INTEGER NOT NULL,
    confirmed_by  TEXT    NOT NULL,
    confirmed_at  BIGINT  NOT NULL,
    PRIMARY KEY (schedule, topic),
    FOREIGN KEY (schedule)     REFERENCES paper_schedules(id) ON DELETE CASCADE,
    FOREIGN KEY (topic)        REFERENCES topics(id)          ON DELETE CASCADE,
    FOREIGN KEY (confirmed_by) REFERENCES users(id)           ON DELETE CASCADE
);

-- ============================================================
-- Paper topics — teacher-selected topics for assessments/
-- assignments (as opposed to the admin-confirmed exam_coverage)
-- weight: higher = more questions drawn from this topic
-- ============================================================
CREATE TABLE paper_topics (
    paper  TEXT    NOT NULL,
    topic  INTEGER NOT NULL,
    weight REAL    NOT NULL DEFAULT 1.0,
    PRIMARY KEY (paper, topic),
    FOREIGN KEY (paper) REFERENCES papers(id) ON DELETE CASCADE,
    FOREIGN KEY (topic) REFERENCES topics(id) ON DELETE CASCADE
);

-- ============================================================
-- Paper questions (new design)
-- paper: FK to papers.id (text)
-- student: NULL = class-wide paper; non-NULL = per-student adm
-- section: 'A', 'B', 'C', 'D', or NULL
-- ============================================================
CREATE TABLE paper_questions (
    paper    TEXT     NOT NULL,
    student  INTEGER,
    question INTEGER  NOT NULL,
    position SMALLINT NOT NULL,
    section  TEXT,
    PRIMARY KEY (paper, student, question),
    FOREIGN KEY (paper)    REFERENCES papers(id)    ON DELETE CASCADE,
    FOREIGN KEY (question) REFERENCES questions(id) ON DELETE CASCADE
);
CREATE INDEX idx_paper_questions_paper ON paper_questions(paper, student);

-- ============================================================
-- Question grades (new design: paper + student + question)
-- awarded_criteria: JSON array of rubric positions awarded
-- ============================================================
CREATE TABLE question_grades (
    paper              TEXT    NOT NULL,
    student            INTEGER NOT NULL,
    question           INTEGER NOT NULL,
    score              REAL    NOT NULL,
    feedback           TEXT,
    awarded_criteria   TEXT,
    created            BIGINT  NOT NULL DEFAULT (unixepoch('now')),
    updated            BIGINT  NOT NULL DEFAULT (unixepoch('now')),
    PRIMARY KEY (paper, student, question),
    FOREIGN KEY (paper)    REFERENCES papers(id)    ON DELETE CASCADE,
    FOREIGN KEY (question) REFERENCES questions(id) ON DELETE CASCADE
);
CREATE INDEX idx_question_grades_paper   ON question_grades(paper);
CREATE INDEX idx_question_grades_student ON question_grades(paper, student);

-- ============================================================
-- Marking queue (new design: single `paper` FK)
-- ============================================================
CREATE TABLE marking_queue (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    paper            TEXT    NOT NULL UNIQUE,
    phase            SMALLINT NOT NULL DEFAULT 0,
    progress         TEXT    NOT NULL DEFAULT '',
    error            TEXT,
    total_students   INTEGER NOT NULL DEFAULT 0,
    marked_students  INTEGER NOT NULL DEFAULT 0,
    created          BIGINT  NOT NULL DEFAULT (unixepoch('now')),
    updated          BIGINT  NOT NULL DEFAULT (unixepoch('now')),
    FOREIGN KEY (paper) REFERENCES papers(id) ON DELETE CASCADE
);

-- ============================================================
-- Grades (redesigned to reference papers.id)
-- ============================================================
CREATE TABLE grades (
    paper   TEXT    NOT NULL,
    student INTEGER NOT NULL,
    score   REAL    NOT NULL,
    created BIGINT  NOT NULL DEFAULT (unixepoch('now')),
    updated BIGINT  NOT NULL DEFAULT (unixepoch('now')),
    PRIMARY KEY (paper, student),
    FOREIGN KEY (paper) REFERENCES papers(id) ON DELETE CASCADE
);
CREATE INDEX idx_grades_paper ON grades(paper);

-- ============================================================
-- Scheme pages (redesigned to reference papers.id)
-- ============================================================
CREATE TABLE scheme_pages (
    paper   TEXT     NOT NULL,
    page    SMALLINT NOT NULL,
    key     TEXT     NOT NULL,
    created BIGINT   NOT NULL DEFAULT (unixepoch('now')),
    PRIMARY KEY (paper, page),
    FOREIGN KEY (paper) REFERENCES papers(id) ON DELETE CASCADE
);

-- ============================================================
-- Answer pages (redesigned to reference papers.id)
-- ============================================================
CREATE TABLE answer_pages (
    paper   TEXT     NOT NULL,
    student INTEGER  NOT NULL,
    page    SMALLINT NOT NULL,
    key     TEXT     NOT NULL,
    created BIGINT   NOT NULL DEFAULT (unixepoch('now')),
    PRIMARY KEY (paper, student, page),
    FOREIGN KEY (paper) REFERENCES papers(id) ON DELETE CASCADE
);
CREATE INDEX idx_answer_pages_paper ON answer_pages(paper, student);

-- ============================================================
-- Per-student PDF keys (generated for per_student mode)
-- ============================================================
CREATE TABLE student_pdf_keys (
    paper        TEXT    NOT NULL,
    student      INTEGER NOT NULL,
    pdf_key      TEXT    NOT NULL,
    generated_at BIGINT  NOT NULL DEFAULT (unixepoch('now')),
    PRIMARY KEY (paper, student),
    FOREIGN KEY (paper) REFERENCES papers(id) ON DELETE CASCADE
);
```

The `down.sql` should drop all new tables and recreate the old ones (copy the DROP sequence from previous migrations). Write the down.sql to simply drop the newly created tables. Full re-creation of old schema in down is not required — just the drops so the migration can be rolled back without data corruption.

```sql
-- 0007 down: remove all tables added in this migration
DROP TABLE IF EXISTS student_pdf_keys;
DROP TABLE IF EXISTS answer_pages;
DROP TABLE IF EXISTS scheme_pages;
DROP TABLE IF EXISTS grades;
DROP TABLE IF EXISTS marking_queue;
DROP TABLE IF EXISTS question_grades;
DROP TABLE IF EXISTS paper_questions;
DROP TABLE IF EXISTS paper_topics;
DROP TABLE IF EXISTS exam_coverage;
DROP TABLE IF EXISTS taught_topics;
DROP TABLE IF EXISTS paper_schedules;
DROP TABLE IF EXISTS papers;
DROP TABLE IF EXISTS events;
DROP TABLE IF EXISTS question_images;
DROP TABLE IF EXISTS part_rubric_criteria;
DROP TABLE IF EXISTS question_parts;
DROP TABLE IF EXISTS rubric_criteria;
DROP TABLE IF EXISTS questions;
```

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task A2: Regenerate Diesel schema.rs
**Files to modify:** `src/db/schema/schema.rs`
**Reference files:** `src/db/schema/schema.rs` (current)
**Depends on:** Task A1
**Parallel group:** sequential (must follow A1)

**Specification:**

Run the following commands from the `ledger/` directory:
```sh
diesel migration run
diesel print-schema > src/db/schema/schema.rs
```

This regenerates `src/db/schema/schema.rs` to reflect all new tables. Do not hand-edit the file. After running, verify the file contains the new tables: `events`, `papers`, `paper_schedules`, `taught_topics`, `exam_coverage`, `paper_topics`, `paper_questions` (new PK), `question_grades` (new PK), `marking_queue` (new single-paper design), `grades` (new), `scheme_pages` (new), `answer_pages` (new), `student_pdf_keys`, `question_parts`, `part_rubric_criteria`.

Remove any stale `diesel::joinable!` and `diesel::allow_tables_to_appear_in_same_query!` entries that reference dropped tables. Add new joinable declarations for all new FK relationships.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

## Track B: Domain Types

### Task B1: Question domain types
**Files to create:** `src/types/question/mod.rs`, `src/types/question/question.rs`, `src/types/question/enums.rs`, `src/types/question/part.rs`, `src/types/question/update.rs`
**Files to modify:** `src/types/mod.rs`
**Reference files:** `src/types/user/user.rs`, `src/types/user/level.rs`, `src/db/schema/schema.rs` (after A2)
**Depends on:** Task A2
**Parallel group:** P_B

**Specification:**

Create `src/types/question/` directory with the following files.

**`src/types/question/enums.rs`** — All question-related enums:

```rust
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::SmallInt;
use diesel::sqlite::Sqlite;

// ── BodyFormat ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum BodyFormat {
    #[default]
    Plain = 0,
    Tiptap = 1,
}

impl TryFrom<i16> for BodyFormat {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Plain),
            1 => Ok(Self::Tiptap),
            _ => Err(crate::types::error::Error::InvalidQuestionText),
        }
    }
}
impl From<BodyFormat> for i16 { fn from(v: BodyFormat) -> i16 { v as i16 } }
impl ToSql<SmallInt, Sqlite> for BodyFormat {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        <i16 as ToSql<SmallInt, Sqlite>>::to_sql(&(*self as i16), out)
    }
}
impl FromSql<SmallInt, Sqlite> for BodyFormat {
    fn from_sql(bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

// ── QuestionType ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum QuestionType {
    #[default]
    Definition     = 0,
    Explanation    = 1,
    Calculation    = 2,
    Structured     = 3,
    Experiment     = 4,
    DataResponse   = 5,
    Diagram        = 6,
}

impl TryFrom<i16> for QuestionType {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Definition), 1 => Ok(Self::Explanation),
            2 => Ok(Self::Calculation), 3 => Ok(Self::Structured),
            4 => Ok(Self::Experiment), 5 => Ok(Self::DataResponse),
            6 => Ok(Self::Diagram),
            _ => Err(crate::types::error::Error::InvalidQuestionText),
        }
    }
}
impl From<QuestionType> for i16 { fn from(v: QuestionType) -> i16 { v as i16 } }
impl ToSql<SmallInt, Sqlite> for QuestionType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        <i16 as ToSql<SmallInt, Sqlite>>::to_sql(&(*self as i16), out)
    }
}
impl FromSql<SmallInt, Sqlite> for QuestionType {
    fn from_sql(bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

// ── CognitiveLevel ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum CognitiveLevel {
    #[default]
    Recall         = 0,
    Comprehension  = 1,
    Application    = 2,
    Analysis       = 3,
}

impl TryFrom<i16> for CognitiveLevel {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Recall), 1 => Ok(Self::Comprehension),
            2 => Ok(Self::Application), 3 => Ok(Self::Analysis),
            _ => Err(crate::types::error::Error::InvalidQuestionText),
        }
    }
}
impl From<CognitiveLevel> for i16 { fn from(v: CognitiveLevel) -> i16 { v as i16 } }
impl ToSql<SmallInt, Sqlite> for CognitiveLevel {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        <i16 as ToSql<SmallInt, Sqlite>>::to_sql(&(*self as i16), out)
    }
}
impl FromSql<SmallInt, Sqlite> for CognitiveLevel {
    fn from_sql(bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

// ── AnswerSpaceType ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum AnswerSpaceType {
    #[default]
    Lines           = 0,
    PlainBox        = 1,
    DiagramBox      = 2,
    ConstructionBox = 3,
    GridBox         = 4,
}

impl TryFrom<i16> for AnswerSpaceType {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Lines), 1 => Ok(Self::PlainBox),
            2 => Ok(Self::DiagramBox), 3 => Ok(Self::ConstructionBox),
            4 => Ok(Self::GridBox),
            _ => Err(crate::types::error::Error::InvalidQuestionText),
        }
    }
}
impl From<AnswerSpaceType> for i16 { fn from(v: AnswerSpaceType) -> i16 { v as i16 } }
impl ToSql<SmallInt, Sqlite> for AnswerSpaceType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        <i16 as ToSql<SmallInt, Sqlite>>::to_sql(&(*self as i16), out)
    }
}
impl FromSql<SmallInt, Sqlite> for AnswerSpaceType {
    fn from_sql(bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

// ── StimulusType ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StimulusType {
    Passage  = 0,
    Table    = 1,
    Graph    = 2,
    Diagram  = 3,
}

// ── ExampleAnswerFormat ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExampleAnswerFormat {
    Plain  = 0,
    Tiptap = 1,
    Svg    = 2,
    Image  = 3,
}
```

**`src/types/question/question.rs`** — Main Question struct:

```rust
use super::enums::*;
use crate::db::schema::questions;
use diesel::{Insertable, Queryable, QueryableByName, Selectable};
use serde::{Deserialize, Serialize};

/// Stored as JSON TEXT in the stimulus column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StimulusImage {
    pub filename:    String,
    pub caption:     String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stimulus {
    #[serde(rename = "type")]
    pub type_:       StimulusType,
    pub body:        String,
    pub body_format: u8,       // 0=plain, 1=tiptap
    pub caption:     String,
    pub image:       Option<StimulusImage>,
}

/// Stored as JSON TEXT in the example_answer column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleAnswerImage {
    pub filename:    String,
    pub caption:     String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleAnswer {
    pub format:  ExampleAnswerFormat,
    pub content: Option<String>,
    pub image:   Option<ExampleAnswerImage>,
}

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = questions)]
pub struct Question {
    pub id:                   Option<i32>,
    pub topic:                i32,
    pub body:                 String,
    pub body_format:          BodyFormat,
    pub stimulus:             Option<String>,   // JSON
    pub type_:                QuestionType,
    pub difficulty:           i16,
    pub cognitive_level:      CognitiveLevel,
    pub marks:                i16,
    pub max_marks:            Option<i16>,
    pub answer_space_type:    AnswerSpaceType,
    pub answer_lines:         Option<i16>,
    pub answer_box_height_mm: Option<i16>,
    pub example_answer:       Option<String>,   // JSON
    pub created:              i64,
    pub updated:              i64,
    pub created_by:           String,
}

/// Changeset for updating a question.
#[derive(Debug, Default, diesel::AsChangeset)]
#[diesel(table_name = questions)]
pub struct QuestionUpdate {
    pub topic:                Option<i32>,
    pub body:                 Option<String>,
    pub body_format:          Option<BodyFormat>,
    pub stimulus:             Option<Option<String>>,
    pub type_:                Option<QuestionType>,
    pub difficulty:           Option<i16>,
    pub cognitive_level:      Option<CognitiveLevel>,
    pub marks:                Option<i16>,
    pub max_marks:            Option<Option<i16>>,
    pub answer_space_type:    Option<AnswerSpaceType>,
    pub answer_lines:         Option<Option<i16>>,
    pub answer_box_height_mm: Option<Option<i16>>,
    pub example_answer:       Option<Option<String>>,
    pub updated:              Option<i64>,
}
```

**`src/types/question/part.rs`** — QuestionPart and PartRubricCriterion structs (see Task B2).

**`src/types/question/mod.rs`**:
```rust
mod enums;
mod part;
mod question;
mod update;

pub use enums::*;
pub use part::*;
pub use question::*;
```

Add `pub mod question;` to `src/types/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task B2: QuestionPart and RubricCriterion domain types
**Files to modify:** `src/types/question/part.rs`
**Reference files:** `src/db/schema/schema.rs` (after A2), `src/types/question/question.rs`
**Depends on:** Task A2, Task B1
**Parallel group:** P_B (can start alongside B1 if B1 creates the module skeleton)

**Specification:**

Add to `src/types/question/part.rs`:

```rust
use crate::db::schema::{part_rubric_criteria, question_parts, rubric_criteria};
use crate::types::question::enums::{AnswerSpaceType, BodyFormat};
use diesel::{Insertable, Queryable, QueryableByName, Selectable};

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = question_parts)]
pub struct QuestionPart {
    pub question:             i32,
    pub position:             i16,
    pub label:                String,
    pub body:                 String,
    pub body_format:          BodyFormat,
    pub marks:                i16,
    pub max_marks:            Option<i16>,
    pub answer_space_type:    AnswerSpaceType,
    pub answer_lines:         Option<i16>,
    pub answer_box_height_mm: Option<i16>,
    pub example_answer:       Option<String>,   // JSON ExampleAnswer
    pub stimulus:             Option<String>,   // JSON Stimulus
}

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = rubric_criteria)]
pub struct RubricCriterion {
    pub question:  i32,
    pub position:  i16,
    pub criterion: String,
    pub marks:     i16,
    pub max_marks: Option<i16>,
    pub required:  bool,
}

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = part_rubric_criteria)]
pub struct PartRubricCriterion {
    pub question:  i32,
    pub part:      i16,
    pub position:  i16,
    pub criterion: String,
    pub marks:     i16,
    pub max_marks: Option<i16>,
    pub required:  bool,
}
```

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task B3: Event domain types
**Files to create:** `src/types/event/mod.rs`, `src/types/event/event.rs`
**Files to modify:** `src/types/mod.rs`
**Reference files:** `src/types/user/level.rs`, `src/db/schema/schema.rs` (after A2)
**Depends on:** Task A2
**Parallel group:** P_B

**Specification:**

Create `src/types/event/` with:

**`src/types/event/event.rs`**:
```rust
use crate::db::schema::events;
use crate::types::id::Id;
use diesel::{Insertable, Queryable, QueryableByName, Selectable};
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::SmallInt;
use diesel::sqlite::Sqlite;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum EventType {
    #[default]
    Exam             = 0,
    Mock             = 1,
    HolidayRevision  = 2,
}

impl TryFrom<i16> for EventType {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v { 0 => Ok(Self::Exam), 1 => Ok(Self::Mock), 2 => Ok(Self::HolidayRevision),
                  _ => Err(crate::types::error::Error::NotFound) }
    }
}
impl From<EventType> for i16 { fn from(v: EventType) -> i16 { v as i16 } }
impl ToSql<SmallInt, Sqlite> for EventType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        <i16 as ToSql<SmallInt, Sqlite>>::to_sql(&(*self as i16), out)
    }
}
impl FromSql<SmallInt, Sqlite> for EventType {
    fn from_sql(bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum EventStatus {
    #[default]
    Draft     = 0,
    Active    = 1,
    Completed = 2,
    Cancelled = 3,
}
// Implement TryFrom<i16>/From/ToSql/FromSql for EventStatus following the same pattern.

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = events)]
pub struct Event {
    pub id:         Id,
    pub school:     String,
    pub name:       String,
    pub type_:      EventType,
    pub term:       i16,
    pub year:       i32,
    pub start_date: i32,
    pub end_date:   i32,
    pub status:     EventStatus,
    pub created:    i64,
    pub updated:    i64,
}

#[derive(Debug, Default, diesel::AsChangeset)]
#[diesel(table_name = events)]
pub struct EventUpdate {
    pub name:       Option<String>,
    pub type_:      Option<EventType>,
    pub term:       Option<i16>,
    pub year:       Option<i32>,
    pub start_date: Option<i32>,
    pub end_date:   Option<i32>,
    pub status:     Option<EventStatus>,
    pub updated:    Option<i64>,
}
```

**`src/types/event/mod.rs`**: re-export all via `pub use event::*;`.

Add `pub mod event;` to `src/types/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task B4: Paper domain types
**Files to create:** `src/types/paper/mod.rs`, `src/types/paper/paper.rs`
**Files to modify:** `src/types/mod.rs`
**Reference files:** `src/types/user/user.rs`, `src/db/schema/schema.rs` (after A2)
**Depends on:** Task A2
**Parallel group:** P_B

**Specification:**

Create `src/types/paper/` with the following enums and structs.

**Enums** (follow the same ToSql/FromSql pattern as Task B3):

```rust
pub enum PaperType {
    Exam        = 0,
    Cat         = 1,
    Assessment  = 2,
    Assignment  = 3,
    Practical   = 4,
    Adaptive    = 5,
}

pub enum PaperStatus {
    Draft        = 0,
    QuestionsSet = 1,
    Finalized    = 2,
    Revealed     = 3,
    Active       = 4,
    Completed    = 5,
    Marked       = 6,
}

pub enum GenerationMode {
    ClassUniform = 0,
    PerStudent   = 1,
}
```

**Paper struct** (uses `papers` table with text PK):

```rust
use crate::db::schema::papers;
use crate::types::id::Id;

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = papers)]
pub struct Paper {
    pub id:               Id,
    pub school:           String,
    pub event:            Option<String>,
    pub subject:          i32,
    pub grade:            i16,
    pub stream:           Option<i16>,
    pub type_:            PaperType,
    pub teacher:          String,
    pub name:             String,
    pub total_marks:      i16,
    pub duration_minutes: i16,
    pub date:             i32,
    pub status:           PaperStatus,
    pub pdf_key:          Option<String>,
    pub ms_key:           Option<String>,
    pub generation_mode:  GenerationMode,
    pub instructions:     Option<String>,
    pub created:          i64,
    pub updated:          i64,
}

#[derive(Debug, Default, diesel::AsChangeset)]
#[diesel(table_name = papers)]
pub struct PaperUpdate {
    pub name:             Option<String>,
    pub event:            Option<Option<String>>,
    pub grade:            Option<i16>,
    pub stream:           Option<Option<i16>>,
    pub type_:            Option<PaperType>,
    pub total_marks:      Option<i16>,
    pub duration_minutes: Option<i16>,
    pub date:             Option<i32>,
    pub status:           Option<PaperStatus>,
    pub pdf_key:          Option<Option<String>>,
    pub ms_key:           Option<Option<String>>,
    pub generation_mode:  Option<GenerationMode>,
    pub instructions:     Option<Option<String>>,
    pub updated:          Option<i64>,
}
```

`src/types/paper/mod.rs` re-exports all. Add `pub mod paper;` to `src/types/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task B5: PaperSchedule domain types
**Files to create:** `src/types/paper/schedule.rs`
**Files to modify:** `src/types/paper/mod.rs`
**Reference files:** `src/db/schema/schema.rs` (after A2)
**Depends on:** Task A2, Task B4
**Parallel group:** P_B

**Specification:**

Add to `src/types/paper/schedule.rs`:

```rust
use crate::db::schema::paper_schedules;
use crate::types::id::Id;

pub enum GenerationStatus {
    Pending    = 0,
    Generating = 1,
    Generated  = 2,
    Failed     = 3,
}
// Implement TryFrom<i16>/From/ToSql/FromSql.

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = paper_schedules)]
pub struct PaperSchedule {
    pub id:                Id,
    pub event:             String,
    pub subject:           i32,
    pub grade:             i16,
    pub stream:            Option<i16>,
    pub date:              i32,
    pub start_time:        i32,   // minutes since midnight
    pub end_time:          i32,   // minutes since midnight
    pub duration_minutes:  i16,
    pub invigilator:       Option<String>,
    pub paper:             Option<String>,
    pub generation_status: GenerationStatus,
    pub reveal_at:         i64,
    pub generate_at:       i64,
    pub created:           i64,
}

#[derive(Debug, Default, diesel::AsChangeset)]
#[diesel(table_name = paper_schedules)]
pub struct PaperScheduleUpdate {
    pub date:              Option<i32>,
    pub start_time:        Option<i32>,
    pub end_time:          Option<i32>,
    pub duration_minutes:  Option<i16>,
    pub invigilator:       Option<Option<String>>,
    pub paper:             Option<Option<String>>,
    pub generation_status: Option<GenerationStatus>,
    pub reveal_at:         Option<i64>,
    pub generate_at:       Option<i64>,
}
```

Re-export from `src/types/paper/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task B6: TaughtTopic, ExamCoverage, PaperTopic domain types
**Files to create:** `src/types/paper/coverage.rs`
**Files to modify:** `src/types/paper/mod.rs`
**Reference files:** `src/db/schema/schema.rs` (after A2)
**Depends on:** Task A2, Task B4
**Parallel group:** P_B

**Specification:**

Add to `src/types/paper/coverage.rs`:

```rust
use crate::db::schema::{exam_coverage, paper_topics, taught_topics};

pub enum TaughtStatus {
    NotStarted = 0,
    InProgress = 1,
    Completed  = 2,
}
// Implement TryFrom<i16>/From/ToSql/FromSql.

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = taught_topics)]
pub struct TaughtTopic {
    pub school:      String,
    pub subject:     i32,
    pub grade:       i16,
    pub stream:      Option<i16>,   // NULL = all streams
    pub topic:       i32,
    pub taught_by:   String,
    pub status:      TaughtStatus,
    pub taught_date: Option<i32>,   // days since epoch
    pub updated:     i64,
}

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = exam_coverage)]
pub struct ExamCoverage {
    pub schedule:     String,
    pub topic:        i32,
    pub confirmed_by: String,
    pub confirmed_at: i64,
}

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = paper_topics)]
pub struct PaperTopic {
    pub paper:  String,
    pub topic:  i32,
    pub weight: f32,
}
```

Re-export from `src/types/paper/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task B7: Error enum additions
**Files to modify:** `src/types/error.rs`
**Reference files:** `src/types/error.rs` (current)
**Depends on:** nothing (can start immediately)
**Parallel group:** P_B

**Specification:**

Add the following variants to the `Error` enum and corresponding `Status` mappings:

**New variants:**
```rust
#[error("event not found")]
EventNotFound,
#[error("paper not found")]
PaperNotFound,
#[error("paper schedule not found")]
PaperScheduleNotFound,
#[error("paper already finalized")]
PaperAlreadyFinalized,
#[error("paper not yet revealed")]
PaperNotRevealed,
#[error("not enough questions for topic allocation")]
NotEnoughQuestionsForAllocation,
#[error("generation in progress")]
GenerationInProgress,
#[error("invalid paper status transition")]
InvalidStatusTransition,
#[error("coverage not confirmed")]
CoverageNotConfirmed,
```

**New `From<Error> for Status` arms:**
```rust
Error::EventNotFound          => Status::not_found("event not found"),
Error::PaperNotFound          => Status::not_found("paper not found"),
Error::PaperScheduleNotFound  => Status::not_found("paper schedule not found"),
Error::PaperAlreadyFinalized  => Status::failed_precondition("paper already finalized"),
Error::PaperNotRevealed       => Status::failed_precondition("paper questions not yet revealed"),
Error::NotEnoughQuestionsForAllocation => Status::failed_precondition(
    "not enough questions in the bank for this topic/mark allocation"),
Error::GenerationInProgress   => Status::failed_precondition("generation already in progress"),
Error::InvalidStatusTransition => Status::failed_precondition("invalid paper status transition"),
Error::CoverageNotConfirmed   => Status::failed_precondition("exam coverage not confirmed by admin"),
```

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task B8: Row structs for new tables
**Files to modify:** `src/db/database/tables/rows.rs`
**Reference files:** `src/db/database/tables/rows.rs` (current, see existing pattern)
**Depends on:** Tasks B3, B4, B5, B6
**Parallel group:** P_B2 (after B3-B6)

**Specification:**

Following the existing pattern in `rows.rs` (each struct has `row_key()` and `school_id()` methods, plus `From<&Row> for XxxInsert`), add row structs for the new tables. The structs are used by the sync watch loop. Use `#[derive(Debug, Clone, QueryableByName)]` and `diesel::sql_query`.

**For `EventRow`:**
```rust
pub struct EventRow {
    pub id:         String,
    pub school:     String,
    pub name:       String,
    pub type_:      i16,
    pub term:       i16,
    pub year:       i32,
    pub start_date: i32,
    pub end_date:   i32,
    pub status:     i16,
    pub created:    i64,
    pub updated:    i64,
}
impl EventRow {
    pub fn row_key(&self) -> String { self.id.clone() }
    pub fn school_id(&self) -> Option<String> { Some(self.school.clone()) }
}
```
(Note: `From<&EventRow> for EventInsert` can be left as `todo!()` until sync.proto is updated in Task D5.)

Add similar row structs for: `PaperRow` (new design with `id TEXT` PK), `PaperScheduleRow`, `TaughtTopicRow`, `ExamCoverageRow`.

For `PaperRow`:
- `row_key()` returns `self.id.clone()`
- `school_id()` returns `Some(self.school.clone())`

For `PaperScheduleRow`:
- `row_key()` returns `self.id.clone()`
- `school_id()`: fetch from event via a query or embed `school` via a join in the snapshot query.
  For now, `school_id()` returns `None` — sync support will be completed in a later task.

For `TaughtTopicRow`:
- `row_key()` returns `format!("{}|{}|{}|{}|{}", school, subject, grade, stream.unwrap_or(-1), topic)`
- `school_id()` returns `Some(self.school.clone())`

**Also update `QuestionRow` struct** to match the new `questions` schema (rename `text` → `body`, add all new columns).

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task B9: LogTable enum update
**Files to modify:** `src/services/sync.rs`
**Reference files:** `src/services/sync.rs` (current, see `LogTable` enum and `resource()` method)
**Depends on:** Task A1
**Parallel group:** P_B

**Specification:**

In `src/services/sync.rs`, add new variants to the `LogTable` enum and update its `from_i32` and `resource()` methods. Append after the existing variants (keeping existing discriminant values stable):

```rust
pub enum LogTable {
    // ... existing variants (Users=1 through AnswerPages=36) ...
    Events         = 37,
    PapersV2       = 38,  // The new papers table (renamed to avoid conflict)
    PaperSchedules = 39,
    TaughtTopics   = 40,
}
```

Update `from_i32()` to handle 37, 38, 39, 40.

Update `resource()` to map the new variants:
- `Events` → `Some(Resource::Classes)` (temporary; adjust if a Papers/Events resource is added to the permission system)
- `PapersV2` → `Some(Resource::Classes)`
- `PaperSchedules` → `Some(Resource::Classes)`
- `TaughtTopics` → `Some(Resource::Classes)`

Update `school_from_key()` for new row key formats:
- `Events` and `PapersV2`: key is the ID, school is in field of the row — return `None` (schoolId fetched separately by snapshot).
- `TaughtTopics`: key is `school|subject|grade|stream|topic` → extract first segment.

Update the `SNAPSHOT_TABLE_ORDER` constant to include the new tables at the end of the slice.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

## Track C: Database Layer

### Task C1: Rewrite question_bank DB operations
**Files to modify:** `src/db/database/tables/question_bank.rs`, `src/db/database/tables/rows.rs`
**Reference files:** `src/db/database/tables/question_bank.rs` (current), `src/db/schema/schema.rs` (after A2)
**Depends on:** Tasks A2, B1, B2
**Parallel group:** P_C

**Specification:**

Rewrite `question_bank.rs` to use the new schema. The following functions need significant changes:

**`insert_question(conn, topic, body, body_format, stimulus, type_, difficulty, cognitive_level, marks, max_marks, answer_space_type, answer_lines, answer_box_height_mm, example_answer, created_by) -> Result<i32>`**
Insert into the new `questions` schema. Return the new auto-increment id.

**`find_or_insert_question(...)`** — same as above but deduplicates on `(topic, body)` using `ON CONFLICT DO NOTHING`.

**`update_question(conn, id, update: QuestionUpdate) -> Result<()>`**
Use Diesel's `update + set()` with the changeset.

**`insert_rubric_criteria(conn, question_id, criteria: &[(i16, String, i16, Option<i16>, bool)]) -> Result<()>`**
Tuple is `(position, criterion, marks, max_marks, required)`.

**`replace_rubric_criteria(conn, question_id, criteria: &[(i16, String, i16, Option<i16>, bool)]) -> Result<()>`**
Delete existing then insert new.

**`get_rubric_criteria(conn, question_id) -> Result<Vec<RubricCriterion>>`**

**`insert_question_parts(conn, question_id, parts: &[NewQuestionPart]) -> Result<()>`**
Where `NewQuestionPart` is a local struct with all part fields.

**`get_question_parts(conn, question_id) -> Result<Vec<QuestionPart>>`**

**`get_part_rubric_criteria(conn, question_id, part_position: i16) -> Result<Vec<PartRubricCriterion>>`**

**`insert_paper_questions(conn, paper_id: &str, student: Option<i32>, questions: &[(i32, i16)]) -> Result<()>`**
Where `questions` is `(question_id, position)`. Now uses new `paper TEXT` + `student INTEGER` schema.

**`get_paper_questions(conn, paper_id: &str, student: Option<i32>) -> Result<Vec<PaperQuestionRow>>`**
Returns rows ordered by position. `student = None` → WHERE student IS NULL.

**`get_full_paper_questions(conn, paper_id: &str, student: Option<i32>) -> Result<Vec<FullPaperQuestion>>`**
Joins questions + rubric + parts.

**`delete_paper_questions(conn, paper_id: &str, student: Option<i32>) -> Result<usize>`**

**`upsert_question_grade(conn, paper_id: &str, student: i32, question_id: i32, score: f32, feedback: Option<&str>, awarded_criteria: Option<&str>) -> Result<()>`**
Uses new `question_grades` PK.

**`get_question_grades_for_student(conn, paper_id: &str, student: i32) -> Result<Vec<QuestionGradeRow>>`**

**`upsert_marking_queue(conn, paper_id: &str) -> Result<()>`**
New schema: just `paper` column, no composite key.

**`update_marking_status(conn, paper_id: &str, phase: i16, progress: &str, error: Option<&str>, total: i32, marked: i32) -> Result<()>`**

**`get_marking_status(conn, paper_id: &str) -> Result<Option<MarkingQueueRow>>`**

**`select_questions_for_paper(conn, topic_id: i32, marks: i16, exclude_ids: &[i32], exclude_recent_student: Option<(i32, usize)>) -> Result<Vec<QuestionRow>>`**
The `exclude_recent_student` parameter is `(student_adm, last_n_papers)` — used during per-student generation to exclude recently seen questions. Query: SELECT q.* FROM questions q WHERE q.topic = ? AND q.marks = ? AND q.id NOT IN (?) AND q.id NOT IN (SELECT question FROM paper_questions pq JOIN papers p ON p.id = pq.paper WHERE pq.student = ? ORDER BY p.date DESC LIMIT ?) ORDER BY RANDOM() LIMIT 30.

Remove all old functions that took `(school, exam, subject, paper, grade, stream)` composite parameters — replace with `paper_id: &str`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task C2: Events database operations
**Files to create:** `src/db/database/tables/events.rs`
**Files to modify:** `src/db/database/tables/mod.rs`
**Reference files:** `src/db/database/tables/question_bank.rs`, `src/db/schema/schema.rs` (after A2)
**Depends on:** Tasks A2, B3
**Parallel group:** P_C

**Specification:**

Create `src/db/database/tables/events.rs` with these functions (all take `conn: &mut SqliteConnection`):

```rust
pub fn insert_event(conn, event: &Event) -> Result<Event>
pub fn get_event(conn, id: &str) -> Result<Option<Event>>
pub fn list_events(conn, school: &str, year: Option<i32>, term: Option<i16>) -> Result<Vec<Event>>
pub fn update_event(conn, id: &str, update: EventUpdate) -> Result<Event>
pub fn delete_event(conn, id: &str) -> Result<bool>  // soft-delete via status=Cancelled
```

Use Diesel ORM for all operations. `insert_event` uses `INSERT INTO events ... RETURNING *` (SQLite 3.35+, enabled by the `returning_clauses_for_sqlite_3_35` feature in Cargo.toml).

`list_events` applies optional filters with `filter()` chains.

Add `pub mod events;` to `src/db/database/tables/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task C3: Papers database operations
**Files to create:** `src/db/database/tables/papers.rs`
**Files to modify:** `src/db/database/tables/mod.rs`
**Reference files:** `src/db/database/tables/events.rs`, `src/db/schema/schema.rs` (after A2)
**Depends on:** Tasks A2, B4, B5, B6
**Parallel group:** P_C

**Specification:**

Create `src/db/database/tables/papers.rs`:

```rust
// Paper CRUD
pub fn insert_paper(conn, paper: &Paper) -> Result<Paper>
pub fn get_paper(conn, id: &str) -> Result<Option<Paper>>
pub fn list_papers(conn, school: &str, event: Option<&str>, grade: Option<i16>,
                   subject: Option<i32>) -> Result<Vec<Paper>>
pub fn update_paper(conn, id: &str, update: PaperUpdate) -> Result<Paper>

// Paper status transitions — enforces valid transitions
// Draft → QuestionsSet → Finalized → Revealed → Active → Completed → Marked
// Returns Error::InvalidStatusTransition on invalid transition.
pub fn transition_paper_status(conn, id: &str, new_status: PaperStatus) -> Result<Paper>

// Paper topic management
pub fn set_paper_topics(conn, paper_id: &str, topics: &[(i32, f32)]) -> Result<()>
pub fn get_paper_topics(conn, paper_id: &str) -> Result<Vec<PaperTopic>>

// Student PDF key management
pub fn upsert_student_pdf_key(conn, paper_id: &str, student: i32, pdf_key: &str) -> Result<()>
pub fn get_student_pdf_key(conn, paper_id: &str, student: i32) -> Result<Option<String>>
pub fn list_student_pdf_keys(conn, paper_id: &str) -> Result<Vec<(i32, String)>>

// Grades
pub fn upsert_grade(conn, paper_id: &str, student: i32, score: f32) -> Result<()>
pub fn get_grade(conn, paper_id: &str, student: i32) -> Result<Option<f32>>
```

The `transition_paper_status` function must validate transitions. Valid next states:
- `Draft` → `QuestionsSet` only
- `QuestionsSet` → `Finalized` or back to `Draft`
- `Finalized` → `Revealed` or back to `QuestionsSet`
- `Revealed` → `Active`
- `Active` → `Completed`
- `Completed` → `Marked`
- Any state → `Draft` if called by admin emergency override (use a separate `force_set_paper_status` function)

```rust
pub fn force_set_paper_status(conn, id: &str, status: PaperStatus) -> Result<Paper>
```

**`get_papers_due_for_reveal(conn) -> Result<Vec<String>>`**
Returns paper IDs for finalized papers whose linked `paper_schedule.reveal_at` has passed.
SQL: `SELECT DISTINCT p.id FROM papers p JOIN paper_schedules ps ON ps.paper = p.id WHERE ps.reveal_at <= unixepoch('now') AND p.status = 2`.
Used by the background scheduler (F5) to auto-transition `Finalized (2) → Revealed (3)`.

**`get_enrolled_students(conn, school: &str, grade: i16, stream: Option<i16>) -> Result<Vec<i32>>`**
Returns all distinct student admission numbers enrolled in the given school+grade combination.
SQL: `SELECT DISTINCT student FROM enrollments WHERE school = ? AND grade = ? AND (? IS NULL OR stream = ?)`.
If `stream` is `None`, returns students across all streams for that grade (used when `papers.stream IS NULL`).
Used by paper finalization (F4's `finalize_student_papers`) and the per-student generation enqueue (F5's `enqueue_assessment` / `enqueue_assignment`).

Add `pub mod papers;` to `src/db/database/tables/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task C4: PaperSchedules, TaughtTopics, ExamCoverage database operations
**Files to create:** `src/db/database/tables/paper_management.rs`
**Files to modify:** `src/db/database/tables/mod.rs`
**Reference files:** `src/db/schema/schema.rs` (after A2), `src/db/database/tables/events.rs`
**Depends on:** Tasks A2, B5, B6
**Parallel group:** P_C

**Specification:**

Create `src/db/database/tables/paper_management.rs`:

```rust
// ── PaperSchedule ──
pub fn insert_schedule(conn, schedule: &PaperSchedule) -> Result<PaperSchedule>
pub fn get_schedule(conn, id: &str) -> Result<Option<PaperSchedule>>
pub fn list_schedules(conn, event_id: &str) -> Result<Vec<PaperSchedule>>
pub fn update_schedule(conn, id: &str, update: PaperScheduleUpdate) -> Result<PaperSchedule>
pub fn assign_invigilator(conn, schedule_id: &str, invigilator: Option<&str>) -> Result<()>
pub fn link_paper_to_schedule(conn, schedule_id: &str, paper_id: &str) -> Result<()>
// Returns schedules due for generation: WHERE generate_at <= now AND generation_status = 0
pub fn get_pending_generation(conn) -> Result<Vec<PaperSchedule>>
pub fn set_generation_status(conn, id: &str, status: GenerationStatus,
                              error: Option<&str>) -> Result<()>

// ── TaughtTopics ──
// Upsert (INSERT OR REPLACE) a taught topic for a teacher
pub fn upsert_taught_topic(conn, topic: &TaughtTopic) -> Result<()>
// Get all taught topics for (school, subject, grade, stream)
pub fn get_taught_topics(conn, school: &str, subject: i32, grade: i16,
                          stream: Option<i16>) -> Result<Vec<TaughtTopic>>
// Get all COMPLETED taught topics for a schedule's subject+grade+stream
// Used by ConfirmExamCoverage to populate exam_coverage from taught_topics.
pub fn get_completed_topics_for_schedule(conn, schedule_id: &str) -> Result<Vec<i32>>
    // topic_ids

// ── ExamCoverage ──
// Replace all coverage entries for a schedule (atomic confirm)
pub fn confirm_exam_coverage(conn, schedule_id: &str, topic_ids: &[i32],
                              confirmed_by: &str) -> Result<usize>
// Get confirmed topics for a schedule (used by generation algorithm)
pub fn get_exam_coverage(conn, schedule_id: &str) -> Result<Vec<i32>>
    // Returns topic_ids
```

Add `pub mod paper_management;` to `src/db/database/tables/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task C5: Update AiMarking database operations
**Files to modify:** `src/db/database/tables/question_bank.rs`
**Reference files:** `src/db/database/tables/question_bank.rs` (after C1)
**Depends on:** Task C1
**Parallel group:** P_C2 (after C1)

**Specification:**

Update the functions used by AiMarking to use the new schema. Specifically:

**`get_question_grades_for_paper(conn, paper_id: &str) -> Result<Vec<QuestionGradeRow>>`**
Returns all student grades for a paper (for aggregation after AI marking).

**`insert_scheme_page(conn, paper_id: &str, page: i16, key: &str) -> Result<()>`**
Inserts into new `scheme_pages(paper, page, key)`.

**`insert_answer_page(conn, paper_id: &str, student: i32, page: i16, key: &str) -> Result<()>`**
Inserts into new `answer_pages(paper, student, page, key)`.

**`get_scheme_pages(conn, paper_id: &str) -> Result<Vec<(i16, String)>>`**
Returns `(page, key)` pairs.

**`get_answer_pages(conn, paper_id: &str, student: i32) -> Result<Vec<(i16, String)>>`**

**`get_paper_student_adms(conn, paper_id: &str) -> Result<Vec<i32>>`**
Returns distinct student adm numbers that have paper_questions rows for this paper.
SQL: `SELECT DISTINCT student FROM paper_questions WHERE paper = ? AND student IS NOT NULL`

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

## Track D: Proto Definitions

### Task D1: Update question_bank.proto
**Files to modify:** `protos/services/question_bank.proto`
**Reference files:** `protos/services/question_bank.proto` (current)
**Depends on:** Tasks B1, B2
**Parallel group:** P_D

**Specification:**

The `Question` message must be replaced with the new schema. Add a `paper_id` field to all school-scoped RPCs (remove `school`, `exam`, `subject`, `paper`, `grade`, `stream` composite keys — replace with single `paper_id: string`).

**New `Question` message:**
```protobuf
message Question {
  int32 id                    = 1;
  int32 topic_id              = 2;
  string body                 = 3;
  int32 body_format           = 4;   // 0=plain, 1=tiptap
  optional string stimulus    = 5;   // JSON Stimulus object
  int32 type                  = 6;   // QuestionType enum
  int32 difficulty            = 7;   // 1..5
  int32 cognitive_level       = 8;   // CognitiveLevel enum
  int32 marks                 = 9;
  optional int32 max_marks    = 10;
  int32 answer_space_type     = 11;  // AnswerSpaceType enum
  optional int32 answer_lines = 12;
  optional int32 answer_box_height_mm = 13;
  optional string example_answer = 14;  // JSON ExampleAnswer object
  repeated RubricCriterion rubric = 15;
  repeated QuestionPart parts     = 16;
  repeated QuestionImage images   = 17;
  int64 created               = 18;
  int64 updated               = 19;
}

message RubricCriterion {
  int32 position          = 1;
  string criterion        = 2;
  int32 marks             = 3;
  optional int32 max_marks = 4;
  bool required           = 5;
}

message QuestionPart {
  int32 position                    = 1;
  string label                      = 2;
  string body                       = 3;
  int32 body_format                 = 4;
  int32 marks                       = 5;
  optional int32 max_marks          = 6;
  int32 answer_space_type           = 7;
  optional int32 answer_lines       = 8;
  optional int32 answer_box_height_mm = 9;
  optional string example_answer    = 10;
  optional string stimulus          = 11;
  repeated RubricCriterion rubric   = 12;
}
```

**Updated school-scoped RPCs** — replace composite key with `paper_id`:

```protobuf
message GeneratePaperRequest {
  string paper_id = 1;
  int32 total_marks = 2;
  repeated TopicAllocation topic_allocations = 3;
}

message FinalizePaperRequest {
  string paper_id = 1;
}

message GetPaperPdfRequest {
  string paper_id = 1;
}

message GetPaperQuestionsRequest {
  string paper_id = 1;
  optional int32 student = 2;  // NULL for class paper
}

message RegenerateQuestionRequest {
  string paper_id      = 1;
  optional int32 student = 2;
  int32 position       = 3;
  int32 topic_id       = 4;
  int32 marks          = 5;
  repeated int32 exclude_ids = 6;
}

message ClearPaperQuestionsRequest {
  string paper_id = 1;
  optional int32 student = 2;
}

message SetPaperQuestionSectionRequest {
  string paper_id = 1;
  optional int32 student = 2;
  int32 position  = 3;
  optional string section = 4;
}

message MarkingStatusRequest {
  string paper_id = 1;
}

message GetQuestionGradesRequest {
  string paper_id = 1;
  int32 student   = 2;
}
```

Add the new `CreateQuestionRequest` fields matching the new Question schema (body_format, stimulus, type, difficulty, cognitive_level, max_marks, answer_space_type, answer_lines, answer_box_height_mm, example_answer, parts):

```protobuf
message CreateQuestionRequest {
  int32 topic_id              = 1;
  string body                 = 2;
  int32 body_format           = 3;
  optional string stimulus    = 4;
  int32 type                  = 5;
  int32 difficulty            = 6;
  int32 cognitive_level       = 7;
  int32 marks                 = 8;
  optional int32 max_marks    = 9;
  int32 answer_space_type     = 10;
  optional int32 answer_lines = 11;
  optional int32 answer_box_height_mm = 12;
  optional string example_answer      = 13;
  repeated RubricCriterionInput rubric = 14;
  repeated QuestionPartInput parts     = 15;
}

message RubricCriterionInput {
  string criterion        = 1;
  int32 marks             = 2;
  optional int32 max_marks = 3;
  bool required           = 4;
}

message QuestionPartInput {
  string label                         = 1;
  string body                          = 2;
  int32 body_format                    = 3;
  int32 marks                          = 4;
  optional int32 max_marks             = 5;
  int32 answer_space_type              = 6;
  optional int32 answer_lines          = 7;
  optional int32 answer_box_height_mm  = 8;
  optional string example_answer       = 9;
  optional string stimulus             = 10;
  repeated RubricCriterionInput rubric = 11;
}
```

Remove the `CopyPaperToStreams` RPC — it is superseded by the new paper scheduling system.

Keep `ListQuestionsRequest`, `GetQuestionRequest`, `BulkImportRequest`, `ImageUploadUrlsRequest` unchanged (these are global catalog operations).

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task D2: New event.proto
**Files to create:** `protos/types/event.proto`, `protos/services/event.proto`
**Reference files:** `protos/services/authentication.proto` (pattern for service definition)
**Depends on:** Task B3
**Parallel group:** P_D

**Specification:**

**`protos/types/event.proto`:**
```protobuf
syntax = "proto3";
package event;

message Event {
  string id         = 1;
  string school     = 2;
  string name       = 3;
  int32 type        = 4;   // 0=exam, 1=mock, 2=holiday_revision
  int32 term        = 5;
  int32 year        = 6;
  int32 start_date  = 7;   // days since epoch
  int32 end_date    = 8;
  int32 status      = 9;   // 0=draft, 1=active, 2=completed, 3=cancelled
  int64 created     = 10;
  int64 updated     = 11;
}
```

**`protos/services/event.proto`:**
```protobuf
syntax = "proto3";
package event_service;
import "types/event.proto";

service EventService {
  rpc CreateEvent(CreateEventRequest)   returns (CreateEventResponse);
  rpc GetEvent(GetEventRequest)         returns (GetEventResponse);
  rpc ListEvents(ListEventsRequest)     returns (ListEventsResponse);
  rpc UpdateEvent(UpdateEventRequest)   returns (UpdateEventResponse);
  rpc DeleteEvent(DeleteEventRequest)   returns (DeleteEventResponse);
}

message CreateEventRequest {
  string school     = 1;
  string name       = 2;
  int32 type        = 3;
  int32 term        = 4;
  int32 year        = 5;
  int32 start_date  = 6;
  int32 end_date    = 7;
}

message CreateEventResponse { event.Event event = 1; }

message GetEventRequest { string event_id = 1; }
message GetEventResponse { event.Event event = 1; }

message ListEventsRequest {
  string school         = 1;
  optional int32 year   = 2;
  optional int32 term   = 3;
}
message ListEventsResponse { repeated event.Event events = 1; }

message UpdateEventRequest {
  string event_id           = 1;
  optional string name      = 2;
  optional int32 type       = 3;
  optional int32 term       = 4;
  optional int32 year       = 5;
  optional int32 start_date = 6;
  optional int32 end_date   = 7;
  optional int32 status     = 8;
}
message UpdateEventResponse { event.Event event = 1; }

message DeleteEventRequest { string event_id = 1; }
message DeleteEventResponse {}
```

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task D3: New paper.proto
**Files to create:** `protos/types/paper.proto`, `protos/services/paper.proto`
**Depends on:** Task B4
**Parallel group:** P_D

**Specification:**

**`protos/types/paper.proto`:**
```protobuf
syntax = "proto3";
package paper;

message Paper {
  string id                = 1;
  string school            = 2;
  optional string event    = 3;
  int32 subject            = 4;
  int32 grade              = 5;
  optional int32 stream    = 6;
  int32 type               = 7;  // PaperType enum
  string teacher           = 8;
  string name              = 9;
  int32 total_marks        = 10;
  int32 duration_minutes   = 11;
  int32 date               = 12;  // days since epoch
  int32 status             = 13;  // PaperStatus enum
  optional string pdf_key  = 14;
  optional string ms_key   = 15;
  int32 generation_mode    = 16;
  optional string instructions = 17;
  int64 created            = 18;
  int64 updated            = 19;
}
```

**`protos/services/paper.proto`:**
```protobuf
syntax = "proto3";
package paper_service;
import "types/paper.proto";

service PaperService {
  rpc CreatePaper(CreatePaperRequest)     returns (CreatePaperResponse);
  rpc GetPaper(GetPaperRequest)           returns (GetPaperResponse);
  rpc ListPapers(ListPapersRequest)       returns (ListPapersResponse);
  rpc UpdatePaper(UpdatePaperRequest)     returns (UpdatePaperResponse);
  rpc GetPaperPdfUrl(GetPaperPdfUrlRequest) returns (GetPaperPdfUrlResponse);
  rpc GetMarkingSchemeUrl(GetMarkingSchemeUrlRequest) returns (GetMarkingSchemeUrlResponse);
  // Admin emergency override: force any status (Super/admin only)
  rpc ForceSetPaperStatus(ForceSetPaperStatusRequest) returns (ForceSetPaperStatusResponse);
}

message CreatePaperRequest {
  string school            = 1;
  optional string event    = 2;
  int32 subject            = 3;
  int32 grade              = 4;
  optional int32 stream    = 5;
  int32 type               = 6;
  string name              = 7;
  int32 total_marks        = 8;
  int32 duration_minutes   = 9;
  int32 date               = 10;
  int32 generation_mode    = 11;
  optional string instructions = 12;
  repeated int32 topic_ids     = 13;  // for assessment/assignment
  repeated PaperTopicWeight topic_weights = 14;
}
message PaperTopicWeight { int32 topic_id = 1; float weight = 2; }
message CreatePaperResponse { paper.Paper paper = 1; }

message GetPaperRequest { string paper_id = 1; }
message GetPaperResponse { paper.Paper paper = 1; }

message ListPapersRequest {
  string school           = 1;
  optional string event   = 2;
  optional int32 grade    = 3;
  optional int32 subject  = 4;
}
message ListPapersResponse { repeated paper.Paper papers = 1; }

message UpdatePaperRequest {
  string paper_id               = 1;
  optional string name          = 2;
  optional int32 total_marks    = 3;
  optional int32 duration_minutes = 4;
  optional int32 date           = 5;
  optional string instructions  = 6;
  optional int32 generation_mode = 7;
}
message UpdatePaperResponse { paper.Paper paper = 1; }

message GetPaperPdfUrlRequest    { string paper_id = 1; }
message GetPaperPdfUrlResponse   { string url = 1; int64 expiry = 2; }
message GetMarkingSchemeUrlRequest { string paper_id = 1; }
message GetMarkingSchemeUrlResponse { string url = 1; int64 expiry = 2; }

message ForceSetPaperStatusRequest { string paper_id = 1; int32 status = 2; }
message ForceSetPaperStatusResponse { paper.Paper paper = 1; }
```

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task D4: New paper_management.proto
**Files to create:** `protos/services/paper_management.proto`
**Depends on:** Tasks B5, B6
**Parallel group:** P_D

**Specification:**

```protobuf
syntax = "proto3";
package paper_management;

service PaperManagement {
  // ── Schedules ──
  rpc SchedulePaper(SchedulePaperRequest)       returns (SchedulePaperResponse);
  rpc AssignInvigilator(AssignInvigilatorRequest) returns (AssignInvigilatorResponse);
  rpc ListPaperSchedules(ListSchedulesRequest)  returns (ListSchedulesResponse);
  rpc UpdateSchedule(UpdateScheduleRequest)     returns (UpdateScheduleResponse);

  // ── Taught Topics ──
  rpc SetTaughtTopics(SetTaughtTopicsRequest)   returns (SetTaughtTopicsResponse);
  rpc GetTaughtTopics(GetTaughtTopicsRequest)   returns (GetTaughtTopicsResponse);

  // ── Coverage (admin step) ──
  rpc ConfirmExamCoverage(ConfirmExamCoverageRequest) returns (ConfirmExamCoverageResponse);
  rpc GetExamCoverage(GetExamCoverageRequest)   returns (GetExamCoverageResponse);

  // ── Generation (teacher triggers) ──
  rpc GenerateAssessment(GenerateAssessmentRequest) returns (GenerateAssessmentResponse);
  rpc GenerateAssignment(GenerateAssignmentRequest) returns (GenerateAssignmentResponse);

  // ── Per-student PDFs ──
  rpc FinalizeStudentPapers(FinalizeStudentPapersRequest) returns (FinalizeStudentPapersResponse);
  rpc GetStudentPapersStatus(GetStudentPapersStatusRequest) returns (GetStudentPapersStatusResponse);
  rpc GetStudentPaperPdf(GetStudentPaperPdfRequest) returns (GetStudentPaperPdfResponse);
}

// ── Schedules ──
message SchedulePaperRequest {
  string event_id            = 1;
  int32 subject              = 2;
  int32 grade                = 3;
  optional int32 stream      = 4;
  int32 date                 = 5;
  int32 start_time           = 6;
  int32 end_time             = 7;
  int32 duration_minutes     = 8;
  optional string invigilator = 9;
  int64 reveal_at            = 10;
  int64 generate_at          = 11;
}
message SchedulePaperResponse {
  string schedule_id         = 1;
}

message AssignInvigilatorRequest {
  string schedule_id         = 1;
  optional string invigilator = 2;
}
message AssignInvigilatorResponse {}

message ListSchedulesRequest { string event_id = 1; }
message PaperScheduleProto {
  string id                  = 1;
  string event               = 2;
  int32 subject              = 3;
  int32 grade                = 4;
  optional int32 stream      = 5;
  int32 date                 = 6;
  int32 start_time           = 7;
  int32 end_time             = 8;
  int32 duration_minutes     = 9;
  optional string invigilator = 10;
  optional string paper      = 11;
  int32 generation_status    = 12;
  int64 reveal_at            = 13;
  int64 generate_at          = 14;
  int64 created              = 15;
}
message ListSchedulesResponse { repeated PaperScheduleProto schedules = 1; }

message UpdateScheduleRequest {
  string schedule_id         = 1;
  optional int32 date        = 2;
  optional int32 start_time  = 3;
  optional int32 end_time    = 4;
  optional int32 duration_minutes = 5;
  optional int64 reveal_at   = 6;
  optional int64 generate_at = 7;
}
message UpdateScheduleResponse { PaperScheduleProto schedule = 1; }

// ── Taught Topics ──
message TaughtTopicProto {
  int32 topic_id             = 1;
  int32 status               = 2;
  optional int32 taught_date = 3;
}
message SetTaughtTopicsRequest {
  string school              = 1;
  int32 subject              = 2;
  int32 grade                = 3;
  optional int32 stream      = 4;
  repeated TaughtTopicProto topics = 5;
}
message SetTaughtTopicsResponse {}

message GetTaughtTopicsRequest {
  string school              = 1;
  int32 subject              = 2;
  int32 grade                = 3;
  optional int32 stream      = 4;
}
message GetTaughtTopicsResponse { repeated TaughtTopicProto topics = 1; }

// ── Coverage ──
message ConfirmExamCoverageRequest {
  string schedule_id         = 1;
  // If topic_ids is empty, server auto-populates from completed taught_topics
  repeated int32 topic_ids   = 2;
}
message ConfirmExamCoverageResponse { int32 topics_confirmed = 1; }

message GetExamCoverageRequest { string schedule_id = 1; }
message GetExamCoverageResponse { repeated int32 topic_ids = 1; }

// ── Generation ──
message GenerateAssessmentRequest {
  string paper_id            = 1;  // paper must exist with type=Assessment
  // generation uses per-student question_grades history
}
message GenerateAssessmentResponse {
  bool accepted              = 1;
  string message             = 2;
}

message GenerateAssignmentRequest {
  string paper_id            = 1;  // paper must exist with type=Assignment
}
message GenerateAssignmentResponse {
  bool accepted              = 1;
  string message             = 2;
}

// ── Per-student PDFs ──
message FinalizeStudentPapersRequest  { string paper_id = 1; }
message FinalizeStudentPapersResponse { string job_id = 1; }

message GetStudentPapersStatusRequest { string paper_id = 1; }
message StudentPdfStatus {
  int32 student     = 1;
  bool  generated   = 2;
  optional string error = 3;
}
message GetStudentPapersStatusResponse {
  string job_id          = 1;
  bool complete          = 2;
  int32 total            = 3;
  int32 generated        = 4;
  repeated StudentPdfStatus statuses = 5;
}

message GetStudentPaperPdfRequest {
  string paper_id        = 1;
  int32 student          = 2;
}
message GetStudentPaperPdfResponse {
  string pdf_url         = 1;
  int64 expiry           = 2;
}
```

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task D5: Register new protos in build.rs and proto/mod.rs
**Files to modify:** `build.rs`, `src/proto/types/mod.rs`, `src/proto/services/mod.rs`
**Reference files:** `build.rs` (current), `src/proto/types/mod.rs` (current)
**Depends on:** Tasks D2, D3, D4
**Parallel group:** P_D2 (after D2-D4)

**Specification:**

In `build.rs`, add the new service proto files to `compile_protos()`:

```rust
configure()
    // ... existing options ...
    .compile_protos(
        &[
            "./protos/services/authentication.proto",
            "./protos/services/sync.proto",
            "./protos/services/ai_marking.proto",
            "./protos/services/question_bank.proto",
            "./protos/types/role.proto",
            "./protos/types/member.proto",
            // New:
            "./protos/services/event.proto",
            "./protos/services/paper.proto",
            "./protos/services/paper_management.proto",
        ],
        &["./protos/"],
    )?;
```

In `src/proto/types/mod.rs`, add:
```rust
pub mod event {
    tonic::include_proto!("event");
}
pub mod paper {
    tonic::include_proto!("paper");
}
```

In `src/proto/services/mod.rs` (if it exists; else create it or add to `src/proto/mod.rs`):
```rust
pub mod event_service;
pub mod paper_service;
pub mod paper_management;
```

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task D6: Update ai_marking.proto
**Files to modify:** `protos/services/ai_marking.proto`
**Reference files:** `protos/services/ai_marking.proto` (current)
**Depends on:** Task D3
**Parallel group:** P_D

**Specification:**

Replace composite `(school, exam, subject, paper, grade, stream)` keys with `paper_id: string` in both RPCs:

```protobuf
syntax = "proto3";
package ai_marking;

service AiMarking {
  rpc RequestUploadUrls(UploadUrlsRequest)   returns (UploadUrlsResponse);
  rpc MarkPaper(MarkPaperRequest)            returns (MarkPaperResponse);
}

message UploadUrlsRequest {
  string paper_id                        = 1;
  int32  scheme_count                    = 2;
  repeated StudentSheetCount students    = 3;
}

message StudentSheetCount { int32 adm = 1; int32 count = 2; }

message UploadUrlsResponse {
  repeated SignedUrl scheme_urls         = 1;
  repeated StudentSignedUrls student_urls = 2;
}

message SignedUrl        { string key = 1; string url = 2; }
message StudentSignedUrls { int32 adm = 1; repeated SignedUrl urls = 2; }

message MarkPaperRequest {
  string paper_id                         = 1;
  int32  total_marks                      = 2;
  repeated string scheme_keys             = 3;
  repeated StudentMarkTarget students     = 4;
}

message StudentMarkTarget { int32 adm = 1; repeated string keys = 2; }

message MarkPaperResponse { bool accepted = 1; string message = 2; }
```

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

## Track E: Proto Adapters

### Task E1: Update question_bank proto adapter
**Files to modify:** `src/proto/services/question_bank.rs`
**Reference files:** `src/proto/services/question_bank.rs` (current), `protos/services/question_bank.proto` (after D1)
**Depends on:** Task D1
**Parallel group:** P_E

**Specification:**

Update the `QuestionBank` trait in `src/proto/services/question_bank.rs` to match the new proto messages from D1.

Key changes to the trait method signatures:
- `generate_paper(&self, token, req: GeneratePaperRequest) -> Result<GeneratePaperResponse>`  
  `req` now has `paper_id: String` instead of composite fields.
- All other school-scoped methods similarly use `paper_id: String`.
- `create_question` now takes the expanded `CreateQuestionRequest` with parts.
- Remove `copy_paper_to_streams` method.
- Add nothing for now regarding the tonic adapter — the adapter impl stays as pass-through.

Update the tonic `impl question_bank_server::QuestionBank for T` block to forward all calls the same way (no parsing needed — all fields are already strings/ints).

Ensure the `extract_token` helper is preserved.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task E2: New event proto adapter
**Files to create:** `src/proto/services/event_service.rs`
**Files to modify:** `src/proto/services/mod.rs`
**Reference files:** `src/proto/services/question_bank.rs` (pattern), `protos/services/event.proto`
**Depends on:** Task D2
**Parallel group:** P_E

**Specification:**

Create `src/proto/services/event_service.rs` following the same adapter pattern:

```rust
tonic::include_proto!("event_service");
pub use event_service_server::EventServiceServer;

pub trait EventService: Sync + Send + 'static + Sized {
    type Config: Sync + Send + 'static;
    fn new(config: Self::Config) -> EventServiceServer<Self>;

    fn create_event(&self, token: Token, req: CreateEventRequest)
        -> impl Future<Output = Result<CreateEventResponse>> + Send;
    fn get_event(&self, token: Token, req: GetEventRequest)
        -> impl Future<Output = Result<GetEventResponse>> + Send;
    fn list_events(&self, token: Token, req: ListEventsRequest)
        -> impl Future<Output = Result<ListEventsResponse>> + Send;
    fn update_event(&self, token: Token, req: UpdateEventRequest)
        -> impl Future<Output = Result<UpdateEventResponse>> + Send;
    fn delete_event(&self, token: Token, req: DeleteEventRequest)
        -> impl Future<Output = Result<DeleteEventResponse>> + Send;
}

#[tonic::async_trait]
impl<T: EventService> event_service_server::EventService for T {
    // Each method: extract_token → call trait method → Response::new
}
```

Add `pub mod event_service;` to `src/proto/services/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task E3: New paper proto adapter
**Files to create:** `src/proto/services/paper_service.rs`
**Files to modify:** `src/proto/services/mod.rs`
**Reference files:** `src/proto/services/event_service.rs`, `protos/services/paper.proto`
**Depends on:** Task D3
**Parallel group:** P_E

**Specification:**

Create `src/proto/services/paper_service.rs` following the same adapter pattern as E2.

Trait methods:
```rust
fn create_paper(&self, token: Token, req: CreatePaperRequest)
    -> impl Future<Output = Result<CreatePaperResponse>> + Send;
fn get_paper(&self, token: Token, req: GetPaperRequest)
    -> impl Future<Output = Result<GetPaperResponse>> + Send;
fn list_papers(&self, token: Token, req: ListPapersRequest)
    -> impl Future<Output = Result<ListPapersResponse>> + Send;
fn update_paper(&self, token: Token, req: UpdatePaperRequest)
    -> impl Future<Output = Result<UpdatePaperResponse>> + Send;
fn get_paper_pdf_url(&self, token: Token, req: GetPaperPdfUrlRequest)
    -> impl Future<Output = Result<GetPaperPdfUrlResponse>> + Send;
fn get_marking_scheme_url(&self, token: Token, req: GetMarkingSchemeUrlRequest)
    -> impl Future<Output = Result<GetMarkingSchemeUrlResponse>> + Send;
fn force_set_paper_status(&self, token: Token, req: ForceSetPaperStatusRequest)
    -> impl Future<Output = Result<ForceSetPaperStatusResponse>> + Send;
```

Add `pub mod paper_service;` to `src/proto/services/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task E4: New paper_management proto adapter and AiMarking update
**Files to create:** `src/proto/services/paper_management.rs`
**Files to modify:** `src/proto/services/ai_marking.rs`, `src/proto/services/mod.rs`
**Reference files:** `src/proto/services/paper_service.rs`, `protos/services/paper_management.proto`, `protos/services/ai_marking.proto` (after D6)
**Depends on:** Tasks D4, D6
**Parallel group:** P_E

**Specification:**

Create `src/proto/services/paper_management.rs` with the `PaperManagement` trait mirroring all RPCs in `paper_management.proto`. Follow the same extract_token + pass-through adapter pattern.

Key trait methods:
```rust
fn schedule_paper(&self, token, req) -> Future<SchedulePaperResponse>;
fn assign_invigilator(&self, token, req) -> Future<AssignInvigilatorResponse>;
fn list_paper_schedules(&self, token, req) -> Future<ListSchedulesResponse>;
fn update_schedule(&self, token, req) -> Future<UpdateScheduleResponse>;
fn set_taught_topics(&self, token, req) -> Future<SetTaughtTopicsResponse>;
fn get_taught_topics(&self, token, req) -> Future<GetTaughtTopicsResponse>;
fn confirm_exam_coverage(&self, token, req) -> Future<ConfirmExamCoverageResponse>;
fn get_exam_coverage(&self, token, req) -> Future<GetExamCoverageResponse>;
fn generate_assessment(&self, token, req) -> Future<GenerateAssessmentResponse>;
fn generate_assignment(&self, token, req) -> Future<GenerateAssignmentResponse>;
fn finalize_student_papers(&self, token, req) -> Future<FinalizeStudentPapersResponse>;
fn get_student_papers_status(&self, token, req) -> Future<GetStudentPapersStatusResponse>;
fn get_student_paper_pdf(&self, token, req) -> Future<GetStudentPaperPdfResponse>;
```

**Update `src/proto/services/ai_marking.rs`** to use the new `MarkPaperRequest` and `UploadUrlsRequest` messages from D6 (replace old composite fields with `paper_id`). The trait method signatures become:
```rust
fn request_upload_urls(&self, token: Token, req: UploadUrlsRequest)
    -> impl Future<Output = Result<UploadUrlsResponse>> + Send;
fn mark_paper(&self, token: Token, req: MarkPaperRequest)
    -> impl Future<Output = Result<MarkPaperResponse>> + Send;
```

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

## Track F: Services

### Task F1: Update QuestionBankService
**Files to modify:** `src/services/question_bank.rs`
**Reference files:** `src/services/question_bank.rs` (current), `src/db/database/tables/question_bank.rs` (after C1), `src/proto/services/question_bank.rs` (after E1)
**Depends on:** Tasks C1, E1
**Parallel group:** P_F

**Specification:**

Rewrite `src/services/question_bank.rs` to use the new schema. Key changes:

**`create_question`**: Now accepts `body`, `body_format`, `stimulus`, `type_`, `difficulty`, `cognitive_level`, `max_marks`, `answer_space_type`, `answer_lines`, `answer_box_height_mm`, `example_answer`, `parts`. After inserting the question, insert parts (with their rubrics) in a transaction.

**`update_question`**: Same extended fields; update parts transactionally.

**`generate_paper`**: Now takes `paper_id: String` instead of composite. Validates the paper exists and is in `Draft` or `QuestionsSet` status. Uses `question_bank::select_questions_for_paper` with the new signature. After generation succeeds, transitions paper status to `QuestionsSet` via `papers::transition_paper_status`. Excludes recently seen questions if the paper's `generation_mode` is `PerStudent`.

**`get_paper_questions`**: Checks paper status >= `Revealed` (status 3) before returning questions. If status < `Revealed`, returns `Error::PaperNotRevealed`. This enforces Decision 44/45. The server-side check: load paper, check `paper.status >= PaperStatus::Revealed as i16`. If called by the teacher who OWNS the paper and the paper is `Finalized` (status 2), allow it (teacher can see their own paper before reveal).

**`finalize_paper`**: Now takes `paper_id`. Loads paper from DB, uses `paper.name` and other metadata for PDF generation. After PDF upload, stores `pdf_key` and `ms_key` on the paper row, then transitions status to `Finalized`. Calls `crate::pdf::generate_paper_pdf_typst` (new function — see Track G).

**`regenerate_question`**: Takes `paper_id` + optional `student`.

**`build_question_proto`**: Updated to include all new Question fields (body, body_format, stimulus, type_, difficulty, cognitive_level, max_marks, answer_space_type, parts, etc.).

**`load_full_question`**: Also loads question_parts and their rubric_criteria.

Remove the `finalize_paper` logic that queries `papers WHERE school=? AND exam=? ...` — replace with `SELECT * FROM papers WHERE id = ?`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task F2: New EventService
**Files to create:** `src/services/events.rs`
**Files to modify:** `src/services/mod.rs`
**Reference files:** `src/services/question_bank.rs`, `src/db/database/tables/events.rs`, `src/proto/services/event_service.rs`
**Depends on:** Tasks C2, E2
**Parallel group:** P_F

**Specification:**

Create `src/services/events.rs`:

```rust
use std::sync::Arc;
use crate::config::Config;
use crate::db::database::CONN;
use crate::db::database::tables::events as events_db;
use crate::proto::services::event_service::*;
use crate::types::error::{Error, Result};
use crate::types::event::{Event, EventUpdate};
use crate::types::id::Id;
use crate::types::token::Token;
use crate::db::database::authorize::authorize_user;
use crate::types::role::{Action, Actions, Organisation, Permissions, Resource};

pub struct EventService<C> { config: Arc<C> }

impl<C: Config + Send + Sync + 'static> EventService<C> for EventService<C> {
    type Config = Arc<C>;

    // CreateEvent — requires Schools.Update (admin creates event for their school)
    async fn create_event(&self, token: Token, req: CreateEventRequest)
        -> Result<CreateEventResponse>
    {
        CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();
            // Authorize: school context, requires Classes.Create permission
            // (using Classes resource as the proxy for exam management)
            let perms = Permissions::from([(Resource::Classes, Actions::from(Action::Create))]);
            authorize_user(conn, &token_to_user(conn, token)?, Organisation::School(req.school.parse()?), perms)?;
            let event = Event {
                id: Id::default(),
                school: req.school,
                name: req.name,
                type_: (req.type_ as i16).try_into()?,
                term: req.term as i16,
                year: req.year,
                start_date: req.start_date,
                end_date: req.end_date,
                status: Default::default(),
                created: chrono::Utc::now().timestamp(),
                updated: chrono::Utc::now().timestamp(),
            };
            let event = events_db::insert_event(conn, &event)?;
            Ok(CreateEventResponse {
                event: Some(event_to_proto(&event)),
            })
        })
    }
    // ... get_event (no auth — any member of school can read),
    // list_events, update_event (requires Classes.Update), delete_event (Classes.Delete)
}
```

Helper `token_to_user(conn, token) -> Result<User>` — decode the PASETO token to get user_id, then load the user row.

Helper `event_to_proto(event: &Event) -> EventProto` — map domain type to proto type.

Add `pub mod events;` to `src/services/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task F3: New PaperService
**Files to create:** `src/services/papers.rs`
**Files to modify:** `src/services/mod.rs`
**Reference files:** `src/services/events.rs`, `src/db/database/tables/papers.rs`, `src/proto/services/paper_service.rs`
**Depends on:** Tasks C3, E3
**Parallel group:** P_F

**Specification:**

Create `src/services/papers.rs` implementing the `PaperService` proto trait.

Key business logic:

**`create_paper`**: Validates that the requesting teacher teaches the given subject in the given school (check `subject_teachers` table for a matching row). Creates the paper in `Draft` status. If `paper_topics` are provided, inserts them into `paper_topics`. Appends changelog record for Papers table.

**`get_paper`**: Loads paper. Checks that user is a member of the paper's school (normal user) or has system access (system user).

**`list_papers`**: Filters by school + optional event/grade/subject. Returns papers visible to the user.

**`update_paper`**: Only allowed when `paper.status == Draft` or `QuestionsSet`. Returns `Error::PaperAlreadyFinalized` if status >= `Finalized`.

**`get_paper_pdf_url`**: Checks `paper.pdf_key` is set (i.e. paper was finalized). Returns a presigned GET URL.

**`get_marking_scheme_url`**: Checks `paper.ms_key`. Returns presigned GET URL. Only visible to the paper's teacher or school admins — if caller is a student, returns `Error::Forbidden`.

**`force_set_paper_status`**: Admin emergency override. Requires Super user level or system-level `Classes.Update` permission. Calls `papers::force_set_paper_status`. Appends changelog record.

Add `pub mod papers;` to `src/services/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task F4: New PaperManagementService
**Files to create:** `src/services/paper_management.rs`
**Files to modify:** `src/services/mod.rs`
**Reference files:** `src/services/papers.rs`, `src/db/database/tables/paper_management.rs`, `src/proto/services/paper_management.rs`
**Depends on:** Tasks C4, E4
**Parallel group:** P_F

**Specification:**

Create `src/services/paper_management.rs` implementing the `PaperManagement` proto trait. Key logic per RPC:

**`schedule_paper`**: Admin-only (requires `Classes.Create`). Creates a `PaperSchedule` record. Returns `schedule_id`. Validates `generate_at < reveal_at < start_time` of the event date.

**`assign_invigilator`**: Admin or school owner. Calls `paper_management::assign_invigilator`.

**`list_paper_schedules`**: Any member of the event's school can read.

**`update_schedule`**: Admin-only. Only allowed if `generation_status == Pending`.

**`set_taught_topics`**: Teacher must teach the given subject in the given school. Calls `paper_management::upsert_taught_topic` for each entry. Each `TaughtTopicProto` sets the status and optional taught_date for one topic.

**`get_taught_topics`**: Any school member can read taught topics for their school.

**`confirm_exam_coverage`**: **Admin-only** (requires `Classes.Update`). If `topic_ids` is empty, auto-populates from `get_completed_topics_for_schedule`. Calls `paper_management::confirm_exam_coverage`. Returns the count of confirmed topics.

**`get_exam_coverage`**: Any school admin can read.

**`generate_assessment`**: Teacher must teach the subject. Paper must exist with `type_ == Assessment` and `status == Draft`. Enqueues per-student generation by calling `crate::services::generation::enqueue_assessment(paper_id)`. Returns `accepted = true` immediately (async job).

**`generate_assignment`**: Same as assessment but `type_ == Assignment`.

**`finalize_student_papers`**: Teacher or admin. Paper must be in `QuestionsSet` status with `generation_mode == PerStudent`. Spawns a background task to generate PDFs for all students enrolled in the paper's school+grade+stream. Returns a `job_id` (use the `paper_id` as the job_id for simplicity — only one finalize job per paper at a time).

**`get_student_papers_status`**: Returns per-student status from `student_pdf_keys` table. `complete = all students have a pdf_key`.

**`get_student_paper_pdf`**: Returns presigned GET URL for the student's PDF key from `student_pdf_keys`.

Add `pub mod paper_management;` to `src/services/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task F5: Auto-generation background job
**Files to create:** `src/services/generation.rs`
**Files to modify:** `src/services/mod.rs`
**Reference files:** `src/services/question_bank.rs`, `src/db/database/tables/question_bank.rs`, `src/db/database/tables/paper_management.rs`, `src/db/database/tables/papers.rs`
**Depends on:** Tasks C1, C3, C4, F1
**Parallel group:** P_F

**Specification:**

Create `src/services/generation.rs` with:

**1. Background scheduler** — spawned as a Tokio task at startup:

```rust
/// Polls paper_schedules WHERE generation_status=0 AND generate_at <= now.
/// Also polls papers WHERE status=Finalized AND linked schedule.reveal_at <= now.
/// Runs every 30 seconds.
pub async fn run_generation_scheduler() {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

        // ── Poll 1: Auto-generate pending exam papers ──────────────────────
        let due = CONN.with(|cell|
            paper_management::get_pending_generation(&mut *cell.borrow_mut())
        );
        match due {
            Ok(schedules) => {
                for schedule in schedules {
                    tokio::spawn(generate_exam_paper(schedule));
                }
            }
            Err(e) => tracing::error!("generation scheduler error: {e}"),
        }

        // ── Poll 2: Auto-reveal finalized papers whose reveal_at has passed ─
        // Transitions papers from Finalized (status=2) → Revealed (status=3)
        // when the paper_schedule.reveal_at timestamp has elapsed.
        let reveal_due = CONN.with(|cell|
            papers::get_papers_due_for_reveal(&mut *cell.borrow_mut())
        );
        match reveal_due {
            Ok(paper_ids) => {
                for paper_id in paper_ids {
                    if let Err(e) = CONN.with(|cell|
                        papers::transition_paper_status(
                            &mut *cell.borrow_mut(),
                            &paper_id,
                            PaperStatus::Revealed,
                        )
                    ) {
                        tracing::error!("auto-reveal failed for paper {paper_id}: {e}");
                    }
                }
            }
            Err(e) => tracing::error!("reveal poll error: {e}"),
        }
    }
}
```

**2. `generate_exam_paper(schedule: PaperSchedule)`** — exam paper generation:

```rust
async fn generate_exam_paper(schedule: PaperSchedule) {
    // 1. Mark generation_status = Generating
    // 2. Load exam_coverage topic_ids for this schedule (admin-confirmed snapshot)
    //    If no coverage exists, fail with alert
    // 3. Create Paper record (insert_paper) with:
    //    - school from event.school
    //    - event = schedule.event
    //    - subject, grade, stream from schedule
    //    - type_ = Exam
    //    - generation_mode = ClassUniform
    //    - status = Draft
    // 4. For each topic_id in coverage, allocate marks proportionally from paper.total_marks
    //    using paper_topics weights if present, otherwise equal distribution
    // 5. Call question_bank::select_questions_for_paper for each topic allocation
    //    (no student exclusion for class-uniform exam papers)
    // 6. Insert paper_questions for student=NULL (class-wide)
    // 7. Generate PDF via crate::pdf::generate_paper_pdf_typst
    // 8. Upload to R2, store pdf_key and ms_key on the paper
    // 9. Transition paper status: Draft → QuestionsSet → Finalized
    // 10. Link paper to schedule: paper_management::link_paper_to_schedule
    // 11. Mark generation_status = Generated
    // On any error: mark generation_status = Failed + store error message
    //   Future: send admin alert notification
}
```

**3. `generate_per_student_paper(paper_id: &str, student_adm: i32, school: &str)`** — per-student generation for assessments/assignments:

```rust
/// Personalized generation algorithm (Decisions 26-30).
///
/// Per-topic score ratio = sum(awarded) / sum(max_marks) for this student.
/// Weak topics (ratio < 0.60): higher question weight, prefer harder questions (difficulty 4-5).
/// Strong topics (ratio >= 0.80): prefer medium difficulty (difficulty 3).
/// No history: neutral weight, standard difficulty (difficulty 3).
/// Exclude questions seen in last 3 papers for this student.
///
/// topic_weights: Vec<(topic_id, weight, preferred_difficulty)>
async fn generate_per_student_paper(paper_id: &str, student_adm: i32) {
    // 1. Load paper (type must be Assessment/Assignment/Adaptive)
    // 2. Load paper_topics for this paper
    // 3. For each topic, query question_grades to compute per-topic score ratio:
    //    SELECT SUM(score), COUNT(*) FROM question_grades qg
    //    JOIN questions q ON q.id = qg.question
    //    WHERE qg.student = ? AND q.topic = ?
    // 4. Determine weight and difficulty preference per topic
    // 5. Load recently seen question_ids for this student (last 3 papers)
    // 6. select_questions_for_paper with exclude_ids from step 5
    // 7. Insert paper_questions with student = student_adm
    // 8. Generate PDF, upload to R2, store in student_pdf_keys
}
```

**4. `enqueue_assessment(paper_id: &str)` / `enqueue_assignment(paper_id: &str)`** — triggered by teacher via GenerateAssessment RPC. These async functions:
- Load enrolled students for the paper's school+grade+stream from the `enrollments` table
- Spawn one Tokio task per student calling `generate_per_student_paper`
- Transition paper status to `QuestionsSet` when all students are done

**5. Startup registration** — handled by Task H1 (`src/server.rs`).
The scheduler is spawned inside `server::start()` before `.serve()`:
```rust
tokio::spawn(crate::services::generation::run_generation_scheduler());
```
Do **not** add this to `src/main.rs`; `main.rs` only calls `server::start().await`.

Add `pub mod generation;` to `src/services/mod.rs`.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task F6: Update AiMarkingService
**Files to modify:** `src/services/ai_marking.rs`
**Reference files:** `src/services/ai_marking.rs` (current), `src/db/database/tables/question_bank.rs` (after C1+C5)
**Depends on:** Tasks C5, E4
**Parallel group:** P_F

**Specification:**

Update `src/services/ai_marking.rs` to use the new proto messages from D6 and new DB functions from C5.

**`request_upload_urls`**: Now takes `paper_id` instead of composite. Use `paper_id` to look up the paper, then build R2 keys using `format!("papers/{}/scheme/page_{}.jpg", paper_id, i)` and `format!("papers/{}/answers/{}/page_{}.jpg", paper_id, student, i)`. Insert scheme_page and answer_page rows.

**`mark_paper`**: Now takes `paper_id`. Use `paper_management::get_paper_student_adms` (or load from request) to get student list. The rest of the AI marking logic (forwarding to AI model, storing question_grades) is unchanged except for using `paper_id` in DB calls.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

## Track G: PDF Engine

### Task G1: Add typst dependency and scaffold new PDF module
**Files to modify:** `Cargo.toml`, `src/pdf.rs`
**Reference files:** `src/pdf.rs` (current — uses printpdf)
**Depends on:** Tasks B1, B2
**Parallel group:** sequential (G1 → G2 → G3)

**Specification:**

**Cargo.toml** — replace `printpdf = "0.9"` with:
```toml
typst = "0.13"
typst-pdf = "0.13"
comemo = "0.4"
```
Keep all other dependencies.

**`src/pdf.rs`** — replace the entire file with a new implementation backed by typst. The public API must remain backwards-compatible for callers in `src/services/question_bank.rs`:

```rust
/// Generate an exam paper PDF using the Typst typesetting engine.
///
/// Returns raw PDF bytes on success.
pub fn generate_paper_pdf_typst(input: &PaperPdfInput) -> Result<Vec<u8>, String>

/// Generate a marking scheme PDF using Typst.
pub fn generate_marking_scheme_pdf_typst(input: &PaperPdfInput) -> Result<Vec<u8>, String>

/// Input structure replacing the old flat parameter list.
pub struct PaperPdfInput<'a> {
    pub school_name:          &'a str,
    pub school_motto:         Option<&'a str>,
    pub paper_name:           &'a str,   // e.g. "Term 2 Exam 2026"
    pub subject_name:         &'a str,
    pub paper_number:         Option<i16>,
    pub grade:                i16,
    pub duration_minutes:     Option<i16>,
    pub instructions:         Option<&'a str>,
    pub questions:            &'a [PaperQuestion],
}

pub struct PaperQuestion {
    pub body:                 String,
    pub body_format:          u8,      // 0=plain, 1=tiptap
    pub marks:                i16,
    pub max_marks:            Option<i16>,
    pub answer_space_type:    u8,
    pub answer_lines:         Option<i16>,
    pub answer_box_height_mm: Option<i16>,
    pub stimulus:             Option<String>,   // JSON Stimulus
    pub example_answer:       Option<String>,   // JSON ExampleAnswer (for marking scheme only)
    pub rubric:               Vec<(String, i16, bool)>,  // (criterion, marks, required)
    pub parts:                Vec<PaperPart>,
    pub section:              Option<String>,
}

pub struct PaperPart {
    pub label:                String,
    pub body:                 String,
    pub body_format:          u8,
    pub marks:                i16,
    pub answer_space_type:    u8,
    pub answer_lines:         Option<i16>,
    pub answer_box_height_mm: Option<i16>,
    pub stimulus:             Option<String>,
    pub rubric:               Vec<(String, i16, bool)>,
}
```

The implementation must compile a Typst `.typ` source string (constructed by the template functions in G2/G3) into PDF bytes. Use a minimal `typst::World` implementation that serves only the single template document (no file system access needed).

The minimal World implementation (required by typst's trait):
```rust
struct SingleDocWorld {
    source: String,
    // typst requires a font provider — use the embedded fonts from typst's std lib
}
impl typst::World for SingleDocWorld {
    // main(): return the source string as the main file
    // source(id): return the source
    // file(id): return Err (no other files)
    // font(index): return from embedded_fonts()
    // today(): return None
    // library(): return typst_library::build()
}
```

Refer to the `typst-as-lib` crate README and `typst::World` documentation for the minimal required implementation. The executor should use whichever approach (direct typst crate or typst-as-lib wrapper) is simpler to implement correctly.

**Note:** The OLD `generate_paper_pdf` and `generate_marking_scheme_pdf` functions (taking flat parameters) can be kept as compatibility wrappers that convert to `PaperPdfInput` and call the new functions. Remove them only after F1 is updated.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task G2: Typst exam paper template
**Files to modify:** `src/pdf.rs`
**Reference files:** `src/pdf.rs` (after G1)
**Depends on:** Task G1
**Parallel group:** sequential (G1 → G2 → G3)

**Specification:**

Add a function `build_exam_paper_typst(input: &PaperPdfInput) -> String` that returns a complete Typst `.typ` source string for the exam paper. The template must:

1. **Header block**: School name (bold, large), school motto (italic, smaller), paper name, subject name, grade, optional paper number, duration, instructions in a bordered box.

2. **Stimulus rendering**: If a question has a `stimulus` JSON field, parse it and render:
   - `passage` type: bordered tinted box (light gray fill, 1pt border) containing the passage body text.
   - `table` type: raw text or a formatted Typst table if body is tab-separated.
   - `graph`/`diagram` types: caption text + placeholder box if no inline image; if `stimulus.image` is set, render an image include directive.
   - SVG inline (for `example_answer.format == svg`): use `#image.decode(svg_data)` in Typst.

3. **Question rendering** (numbered, bold question number):
   - Question body (plain or tiptap — for tiptap, strip tags and render as plain text for now; full tiptap→typst conversion is a follow-on task).
   - For each **part**: `(a)`, `(b)`, etc. with the part body, marks, and answer space.
   - After each question (or part), render **answer space** according to `answer_space_type`:
     - `Lines (0)`: `answer_lines` horizontal ruled lines (`#line(length: 100%)` repeated).
     - `PlainBox (1)` / `DiagramBox (2)`: a `#rect(width: 100%, height: Xmm)` where X = `answer_box_height_mm` (default 40).
     - `ConstructionBox (3)`: same as DiagramBox but labeled "Construction space".
     - `GridBox (4)`: a Typst grid pattern rect.
   - Marks in brackets at right margin: `[X marks]`.
   - **Section headers**: when `section` changes, insert a bold centered "SECTION A", "SECTION B" etc.

4. **Page numbering**: `#set page(numbering: "1 / 1")`.

5. **Fonts**: Use `#set text(font: "Linux Libertine")` (built into Typst's embedded fonts) or fall back to `"New Computer Modern"`.

A minimal template skeleton:

```typst
#set page(paper: "a4", margin: (top: 20mm, bottom: 25mm, left: 20mm, right: 20mm),
          numbering: "1")
#set text(font: "New Computer Modern", size: 11pt)
#set par(justify: true)

// Header
#align(center)[
  #text(weight: "bold", size: 14pt)[SCHOOL_NAME]
  #if MOTTO != "" [ \ #text(style: "italic", size: 10pt)[MOTTO] ]
  \ #text(weight: "bold")[PAPER_NAME]
  \ #text()[SUBJECT_NAME — Grade GRADE#if PAPER_NUM != "" [ — Paper PAPER_NUM]]
  \ #if DURATION != "" [Time Allowed: DURATION minutes]
]
#line(length: 100%)
// Instructions box
#block(stroke: 1pt, inset: 8pt, width: 100%)[
  *Instructions:* \
  INSTRUCTIONS_TEXT
]
#v(5mm)
// Questions ...
```

The function should build this template by string interpolation, filling in the actual values. For each question, append the appropriate Typst markup.

The `generate_paper_pdf_typst(input)` function calls `build_exam_paper_typst(input)` to get the source string, then compiles it via the `SingleDocWorld` from G1.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

### Task G3: Typst marking scheme template and per-student PDF generation
**Files to modify:** `src/pdf.rs`, `src/services/generation.rs`
**Reference files:** `src/pdf.rs` (after G2), `src/services/generation.rs` (after F5)
**Depends on:** Task G2, Task F5
**Parallel group:** sequential (last in G track)

**Specification:**

**Part 1 — Marking scheme template:**

Add `build_marking_scheme_typst(input: &PaperPdfInput) -> String`. The marking scheme template differs from the exam paper in:
- Title: "MARKING SCHEME" appended to subject/paper header.
- No answer space rendered (omit ruled lines/boxes).
- After each question body, render the **rubric** criteria as a numbered list:
  - Each criterion: `[position]. criterion_text ... [marks mark(s)]`
  - If `required = true`, prefix with `*` (must be awarded).
  - If `max_marks` is set on the question, add: `(award any [max_marks] of the following)`.
- Render `example_answer` below the rubric:
  - `format = plain`: render as plain text.
  - `format = tiptap`: strip HTML/tags, render as plain text.
  - `format = svg`: use `#image.decode(svg_data, format: "svg")`.
  - `format = image`: render `[See image: filename]`.
- For parts: each part renders its own rubric + example_answer.

The `generate_marking_scheme_pdf_typst(input)` function calls `build_marking_scheme_typst(input)` then compiles.

**Part 2 — Per-student named PDF:**

Add `build_student_exam_paper_typst(input: &PaperPdfInput, student_name: &str, student_adm: i32) -> String`. This is identical to the exam paper template but adds student name and admission number pre-filled in the header:

```typst
// Below the header, before instructions:
#block(stroke: 0.5pt, inset: 6pt)[
  Name: #underline[STUDENT_NAME] #h(1fr) Adm No: #underline[ADM_NO]
]
```

**Part 3 — Finalize student papers logic in generation.rs:**

In `finalize_student_papers` (called from F4's `FinalizeStudentPapersService`), the per-student PDF generation loop:

```rust
// For each student enrolled in paper's school+grade+stream:
//   1. Load their paper_questions (student-specific if generation_mode=PerStudent,
//      or class-wide paper_questions if ClassUniform)
//   2. Build PaperPdfInput from the questions
//   3. Call generate_paper_pdf_typst (named variant with student info)
//   4. Upload to R2: key = format!("papers/{}/students/{}.pdf", paper_id, student_adm)
//   5. Call papers::upsert_student_pdf_key(paper_id, student_adm, &key)
```

For `ClassUniform` mode: all students get the same questions but a named PDF (student name + adm pre-filled). For `PerStudent` (adaptive): each student has their own `paper_questions` rows.

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

## Track H: Server Wiring

### Task H1: Wire all new services in server.rs
**Files to modify:** `src/server.rs`, `src/services/mod.rs`, `src/proto/services/mod.rs`
**Reference files:** `src/server.rs` (current), `src/services/events.rs`, `src/services/papers.rs`, `src/services/paper_management.rs`, `src/services/generation.rs`
**Depends on:** Tasks F1, F2, F3, F4, F5, F6, G1, G2, G3
**Parallel group:** sequential (last task)

**Specification:**

Update `src/server.rs` to register all new services and start the generation scheduler:

```rust
use crate::proto::services::event_service::EventService;
use crate::proto::services::paper_service::PaperService;
use crate::proto::services::paper_management::PaperManagement;
use crate::services::events::EventService as EventServiceImpl;
use crate::services::papers::PaperServiceImpl;
use crate::services::paper_management::PaperManagementServiceImpl;

pub async fn start() -> Result<()> {
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 50051));
    let config = Arc::new(Configuration::default());

    // ... existing startup changelog records ...

    // Start the auto-generation background scheduler
    tokio::spawn(crate::services::generation::run_generation_scheduler());

    let authenticator    = Authenticator::new(config.clone());
    let sync             = SyncService::new(config.clone());
    let ai_marking       = AiMarkingService::new(config.clone());
    let question_bank    = QuestionBankService::new(config.clone());
    let event_service    = EventServiceImpl::new(config.clone());
    let paper_service    = PaperServiceImpl::new(config.clone());
    let paper_management = PaperManagementServiceImpl::new(config.clone());

    Server::builder()
        .add_service(authenticator)
        .add_service(sync)
        .add_service(ai_marking)
        .add_service(question_bank)
        .add_service(event_service)
        .add_service(paper_service)
        .add_service(paper_management)
        .serve(addr)
        .await?;
    Ok(())
}
```

Ensure `src/services/mod.rs` has all new modules declared:
```rust
pub mod ai_marking;
pub mod authentication;
pub mod events;
pub mod generation;
pub mod paper_management;
pub mod papers;
pub mod question_bank;
pub mod sync;
```

Ensure `src/proto/services/mod.rs` has all adapters declared:
```rust
pub mod ai_marking;
pub mod authentication;
pub mod event_service;
pub mod paper_management;
pub mod paper_service;
pub mod question_bank;
pub mod sync;
```

**Update after completion:**
- [ ] Mark this task `[x]`
- [ ] Orchestrator: git commit after this task

---

## Design Decision Coverage Matrix

| Decision | Tasks |
|---|---|
| 1. body_format | A1, B1, D1 |
| 2. stimulus on question+part | A1, B1, B2, D1, G2, G3 |
| 3. parts array | A1, B2, C1, D1, G2, G3 |
| 4. QuestionType enum | A1, B1, D1 |
| 5. difficulty | A1, B1, D1 |
| 6. cognitive_level | A1, B1, D1 |
| 7. atomic rubric + max_marks | A1, B1, B2, D1 |
| 8. answer_space_type | A1, B1, B2, D1, G2 |
| 9. answer_lines | A1, B1, B2, D1, G2 |
| 10. answer_box_height_mm | A1, B1, B2, D1, G2 |
| 11. example_answer typed object | A1, B1, D1, G3 |
| 12. rubric_criteria max_marks+required | A1, B2, D1 |
| 13. backend clean slate | A1 |
| 14. events table | A1, B3, C2, D2, E2, F2 |
| 15. papers new design | A1, B4, C3, D3, E3, F3 |
| 16. paper_schedules | A1, B5, C4, D4, E4, F4 |
| 17. taught_topics | A1, B6, C4, D4, E4, F4 |
| 18. exam_coverage | A1, B6, C4, D4, E4, F4 |
| 19. paper_topics | A1, B6, C4, D4 |
| 20. paper_questions redesign | A1, C1 |
| 21. question_grades redesign | A1, C1 |
| 22. marking_queue redesign | A1, C1, C5 |
| 23. student_pdf_keys | A1, C3, G3 |
| 24. auto-generation at generate_at | F5 |
| 25. generation reads exam_coverage | F5 |
| 26. per-student generation from question_grades | F5, G3 |
| 27. recently-seen question exclusion | C1, F5 |
| 28. exam results as primary personalization signal | F5 |
| 29. no history = neutral weight | F5 |
| 30. generation failure alert | F5 |
| 31. background job/scheduler | F5, H1 |
| 32. typst PDF | G1, G2, G3 |
| 33. three PDF tiers | G2, G3 |
| 34. per-student PDFs + RPCs | D4, E4, F4, G3 |
| 35. typst templates with stimulus/SVG | G2, G3 |
| 36. answer_space_type in PDF | G2, G3 |
| 37. paper_id replaces composite keys | D1, D3, D4, D6, E1, E4, F1, F6 |
| 38. CreateEvent/GetEvent/ListEvents/UpdateEvent | D2, E2, F2 |
| 39. CreatePaper/GetPaper/ListPapers/UpdatePaper | D3, E3, F3 |
| 40. SchedulePaper/AssignInvigilator/ListPaperSchedules | D4, E4, F4 |
| 41. SetTaughtTopics/GetTaughtTopics/ConfirmExamCoverage | D4, E4, F4 |
| 42. GenerateAssessment/GenerateAssignment | D4, E4, F4, F5 |
| 43. FinalizeStudentPapers/GetStudentPapersStatus/GetStudentPaperPdf | D4, E4, F4, G3 |
| 44. GetPaperQuestions requires status >= Revealed | F1 |
| 45. teacher blocked before reveal_at | F1 |
| 46. admin emergency override | D3, E3, F3 |
| 47. only admin can ConfirmExamCoverage/SchedulePaper | F4 |
| 48. teacher can generate for their subjects | F4, F5 |
| 49. reveal_at auto-transition (Finalized → Revealed) | C3, F5 |
| 50. enrolled students lookup for per-student generation | C3, F4, F5 |

---

## Task Checklist (all tasks)

- [ ] A1: Migration SQL
- [ ] A2: Regenerate schema.rs
- [ ] B1: Question domain types
- [ ] B2: QuestionPart domain types
- [ ] B3: Event domain types
- [ ] B4: Paper domain types
- [ ] B5: PaperSchedule domain types
- [ ] B6: TaughtTopic/ExamCoverage/PaperTopic types
- [ ] B7: Error enum additions
- [ ] B8: Row structs for new tables
- [ ] B9: LogTable enum update
- [ ] C1: Rewrite question_bank DB ops
- [ ] C2: Events DB ops
- [ ] C3: Papers DB ops
- [ ] C4: PaperSchedules/TaughtTopics/ExamCoverage DB ops
- [ ] C5: AiMarking DB ops update
- [ ] D1: Update question_bank.proto
- [ ] D2: New event.proto
- [ ] D3: New paper.proto
- [ ] D4: New paper_management.proto
- [ ] D5: Register protos in build.rs
- [ ] D6: Update ai_marking.proto
- [ ] E1: Update question_bank proto adapter
- [ ] E2: New event proto adapter
- [ ] E3: New paper proto adapter
- [ ] E4: New paper_management proto adapter + ai_marking update
- [ ] F1: Update QuestionBankService
- [ ] F2: New EventService
- [ ] F3: New PaperService
- [ ] F4: New PaperManagementService
- [ ] F5: Auto-generation background job
- [ ] F6: Update AiMarkingService
- [ ] G1: Add typst dependency + scaffold
- [ ] G2: Typst exam paper template
- [ ] G3: Typst marking scheme + per-student PDFs
- [ ] H1: Wire all services in server.rs