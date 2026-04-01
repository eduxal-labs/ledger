-- Question bank tables (server-only — NOT synced to clients)

CREATE TABLE questions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    topic INTEGER NOT NULL,
    text TEXT NOT NULL,
    marks SMALLINT NOT NULL,
    example_answer TEXT,
    created BIGINT NOT NULL DEFAULT (unixepoch('now')),
    updated BIGINT NOT NULL DEFAULT (unixepoch('now')),
    created_by TEXT NOT NULL,
    FOREIGN KEY (topic) REFERENCES topics(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX idx_questions_topic ON questions(topic);
CREATE INDEX idx_questions_topic_marks ON questions(topic, marks);

CREATE TABLE rubric_criteria (
    question INTEGER NOT NULL,
    position SMALLINT NOT NULL,
    criterion TEXT NOT NULL,
    marks SMALLINT NOT NULL,
    PRIMARY KEY (question, position),
    FOREIGN KEY (question) REFERENCES questions(id) ON DELETE CASCADE
);

CREATE TABLE question_images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    question INTEGER NOT NULL,
    position SMALLINT NOT NULL,
    context SMALLINT NOT NULL,   -- 0=question, 1=rubric, 2=example_answer
    key TEXT NOT NULL,
    caption TEXT,
    FOREIGN KEY (question) REFERENCES questions(id) ON DELETE CASCADE
);
CREATE INDEX idx_question_images_question ON question_images(question);

CREATE TABLE question_grades (
    school TEXT NOT NULL,
    exam TEXT NOT NULL,
    student INTEGER NOT NULL,
    question INTEGER NOT NULL,
    score REAL NOT NULL,
    feedback TEXT,
    created BIGINT NOT NULL DEFAULT (unixepoch('now')),
    updated BIGINT NOT NULL DEFAULT (unixepoch('now')),
    PRIMARY KEY (school, exam, student, question),
    FOREIGN KEY (school) REFERENCES schools(id) ON DELETE CASCADE,
    FOREIGN KEY (exam) REFERENCES exams(id) ON DELETE CASCADE,
    FOREIGN KEY (school, student) REFERENCES students(school, adm) ON DELETE CASCADE,
    FOREIGN KEY (question) REFERENCES questions(id) ON DELETE CASCADE
);
CREATE INDEX idx_question_grades_student ON question_grades(school, student);
CREATE INDEX idx_question_grades_exam ON question_grades(school, exam);

CREATE TABLE paper_questions (
    school TEXT NOT NULL,
    exam TEXT NOT NULL,
    subject INTEGER NOT NULL,
    paper SMALLINT,
    grade SMALLINT NOT NULL,
    stream SMALLINT,
    question INTEGER NOT NULL,
    position SMALLINT NOT NULL,
    PRIMARY KEY (school, exam, subject, paper, grade, stream, question),
    FOREIGN KEY (question) REFERENCES questions(id) ON DELETE CASCADE
);

CREATE TABLE marking_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    school TEXT NOT NULL,
    exam TEXT NOT NULL,
    subject INTEGER NOT NULL,
    paper SMALLINT,
    grade SMALLINT NOT NULL,
    stream SMALLINT,
    phase SMALLINT NOT NULL DEFAULT 0,        -- MarkingPhase enum (0-6)
    progress TEXT NOT NULL DEFAULT '',         -- e.g. "5/30 students marked"
    error TEXT,
    total_students INTEGER NOT NULL DEFAULT 0,
    marked_students INTEGER NOT NULL DEFAULT 0,
    created BIGINT NOT NULL DEFAULT (unixepoch('now')),
    updated BIGINT NOT NULL DEFAULT (unixepoch('now')),
    FOREIGN KEY (school) REFERENCES schools(id) ON DELETE CASCADE,
    FOREIGN KEY (exam) REFERENCES exams(id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX idx_marking_queue_paper ON marking_queue(school, exam, subject, paper, grade, stream);
