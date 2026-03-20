-- File sync metadata tables: scheme pages and answer sheet pages.
-- These track S3 object keys for marking scheme images and student answer sheet images.

CREATE TABLE scheme_pages (
    school text not null,
    exam text not null,
    subject integer not null,
    paper smallint, -- NULL = single-paper subject
    page smallint not null, -- 0-indexed page number
    key text not null, -- S3 object key
    created bigint not null default (unixepoch('now')),
    primary key (school, exam, subject, paper, page),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (exam) references exams(id) ON DELETE CASCADE,
    foreign key (subject) references subjects(id) ON DELETE CASCADE
);

CREATE TABLE answer_pages (
    school text not null,
    exam text not null,
    student integer not null,
    subject integer not null,
    paper smallint, -- NULL = single-paper subject
    page smallint not null, -- 0-indexed page number
    key text not null, -- S3 object key
    created bigint not null default (unixepoch('now')),
    primary key (school, exam, student, subject, paper, page),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (exam) references exams(id) ON DELETE CASCADE,
    foreign key (school, student) references students(school, adm) ON DELETE CASCADE,
    foreign key (subject) references subjects(id) ON DELETE CASCADE
);

-- Performance indexes for bulk lookups (e.g. "all scheme pages for this paper")
CREATE INDEX idx_scheme_pages_paper ON scheme_pages(school, exam, subject, paper);
CREATE INDEX idx_answer_pages_student ON answer_pages(school, exam, student, subject, paper);
