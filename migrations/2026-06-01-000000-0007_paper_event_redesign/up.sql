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
-- taught topics for a specific paper_schedule.
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
-- Paper topics — teacher-selected topics for assessments/assignments
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
-- student: NULL = class-wide paper; non-NULL = per-student
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
