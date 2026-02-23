-- there are 2 types of time here:
-- DateTime => This is represented as bigint which are seconds since epoch
-- Date => This is represented as integer which are days since epoch
-- We should make good use of enums which are represented as smallints in the database.

CREATE TABLE users (
    id text primary key not null,
    phone text not null unique,
    email text,
    name text not null,
    level smallint not null default 0, -- either Normal = 0, System = 1, Super = 2
    status smallint not null default 0, -- either Invited = 0, Active = 1, Suspended = 2, Deleted = 3
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now'))
);

CREATE TABLE schools (
    id text primary key not null,
    name text not null,
    motto text,
    phone text,
    email text,
    county integer not null,
    domain text,
    established integer,
    status smallint not null default 0, -- either Trial = 0, Active = 1, Cancelled = 2, Suspended = 3, Deleted = 4,
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now'))
);

CREATE TABLE owners (
    school text not null,
    user text not null,
    created bigint not null default (unixepoch('now')),
    primary key (school, user),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (user) references users(id) ON DELETE CASCADE
);

CREATE TABLE students (
    school text not null,
    adm integer not null,
    user text,
    name text not null,
    dob integer,
    gender smallint CHECK (gender IN (0, 1)), -- Male = 0, Female = 1
    documents text,
    admitted integer,
    status smallint not null default 0, -- either Active = 0, Expelled = 1, Graduated = 2, Transferred = 3, Withdrawn = 4, Deleted = 5
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    primary key (school, adm),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (user) references users(id) ON DELETE SET NULL
);
CREATE UNIQUE INDEX students_school_user_idx ON students(school, user) WHERE user IS NOT NULL;

CREATE TABLE guardians (
    school text not null,
    user text not null,
    student integer not null,
    relationship smallint not null, -- either Father = 0, Mother = 1, Brother = 2, Sister = 3, Guardian = 4.
    role smallint not null default 0, -- either Primary = 0, Secondary = 1, Sponsor = 2.
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    primary key (school, user, student),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (user) references users(id) ON DELETE CASCADE,
    foreign key (school, student) references students(school, adm) ON DELETE CASCADE
);

CREATE UNIQUE INDEX uq_guardians_primary ON guardians(school, student) WHERE role = 0;

CREATE TABLE departments (
    school text not null,
    name text not null,
    description text,
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    primary key (school, name),
    foreign key (school) references schools(id) ON DELETE CASCADE
);

CREATE TABLE teachers (
    school text not null,
    user text not null,
    hired integer,
    role text,
    department text,
    status smallint not null default 0, -- either Active = 0, Resigned = 1, Transferred = 2, Fired = 3, Retired = 4.
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    primary key (school, user),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (user) references users(id) ON DELETE CASCADE,
    foreign key (school, department) references departments(school, name) ON DELETE NO ACTION
);

CREATE TABLE staff (
    school text not null,
    user text not null,
    idnumber text,
    role text,
    department text,
    status smallint not null default 0, -- either Active = 0, Resigned = 1, Transferred = 2, Fired = 3, Retired = 4.
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    primary key (school, user),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (user) references users(id) ON DELETE CASCADE,
    foreign key (school, department) references departments(school, name) ON DELETE NO ACTION
);


CREATE TABLE terms (
    school text not null,
    year integer not null,
    term smallint not null,
    start bigint not null,
    end bigint not null,
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    CHECK (start < end),
    primary key (school, year, term),
    foreign key (school) references schools(id) ON DELETE CASCADE
);

CREATE TABLE class_teachers (
    school text not null,
    year integer not null,
    term smallint not null,
    grade smallint not null,
    stream smallint not null,
    teacher text not null,
    start integer not null default (unixepoch('now') / 86400),
    end integer, -- this can only be null for the currently active teacher. if a new one is created the end of the previous has to be populated.
    created bigint not null default (unixepoch('now')),
    primary key (school, year, term, grade, stream, teacher), -- I would love if I could include the (start..end) range into the primary key so that we could have a history of the class teachers over time and end should only be null for the most recent and the most active teacher.
    foreign key (school) references schools(id) ON DELETE CASCADE,
    CHECK (end IS NULL OR start < end),
    foreign key (school, teacher) references teachers(school, user) ON DELETE CASCADE,
    foreign key (school, year, term) references terms(school, year, term) ON DELETE CASCADE
);

-- students to their class existence.
CREATE TABLE enrollments (
    school text not null,
    year integer not null,
    term smallint not null,
    grade smallint not null, -- either Grade 1, Grade 2, Grade 3, Grade 4 and so on. we will use 43 for fomr3 and 44 for form4.
    stream smallint not null, -- streams are represented in numbers but each school will decide which name is assigned to which stream number.
    student integer not null,
    created bigint not null default (unixepoch('now')),
    primary key (school, year, term, grade, stream, student),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (school, student) references students(school, adm) ON DELETE CASCADE,
    foreign key (school, year, term) references terms(school, year, term) ON DELETE CASCADE
);
-- A student can only be in one class (grade+stream) per term.
CREATE UNIQUE INDEX uq_enrollments_student_term ON enrollments(school, year, term, student);

-- since subjects will obviously be known, they will be represented by an integer.
-- this table will represent the subjects taught in a class and by which teacher.
CREATE TABLE subjects (
    school text not null,
    year integer not null,
    term smallint not null,
    grade smallint not null,
    stream smallint not null,
    subject smallint not null,
    teacher text not null,
    created bigint not null default (unixepoch('now')),
    primary key (school, year, term, grade, stream, subject),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (school, year, term) references terms(school, year, term) ON DELETE CASCADE,
    foreign key (school, teacher) references teachers(school, user) ON DELETE CASCADE
);
-- Needed so timetable can FK on the (subject, teacher) pair to enforce teacher consistency.
-- Since (school, year, term, grade, stream, subject) is already the PK, this 7-column index
-- is trivially unique but gives SQLite a target for the composite FK reference.
CREATE UNIQUE INDEX subjects_class_teacher_idx ON subjects(school, year, term, grade, stream, subject, teacher);

CREATE TABLE attendance (
    school text not null,
    year integer not null,
    term smallint not null,
    grade smallint not null,
    stream smallint not null,
    student integer not null,
    date integer not null,
    status smallint not null, -- either Present = 1, Absent = 2, Leave = 3
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    primary key (school, year, term, grade, stream, student, date),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (school, year, term) references terms(school, year, term) ON DELETE CASCADE,
    foreign key (school, year, term, grade, stream, student) references enrollments(school, year, term, grade, stream, student) ON DELETE CASCADE
);

CREATE TABLE timetable (
    school text not null,
    year integer not null,
    term smallint not null,
    grade smallint not null,
    stream smallint not null,
    subject smallint not null,
    teacher text not null,
    day smallint not null, -- Sunday = 0, .. Saturday = 6.
    start integer not null, -- seconds since midnight
    end integer not null, -- seconds since midnight
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    CHECK (start < end),
    primary key (school, year, term, grade, stream, day, subject, start),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (school, year, term) references terms(school, year, term) ON DELETE CASCADE,
    -- FK on (subject, teacher) together enforces that the timetable teacher matches the assigned
    -- subject teacher. ON UPDATE CASCADE keeps timetable in sync if the teacher is reassigned.
    foreign key (school, year, term, grade, stream, subject, teacher) references subjects(school, year, term, grade, stream, subject, teacher) ON DELETE CASCADE ON UPDATE CASCADE
);
CREATE UNIQUE INDEX uq_timetable_teacher_slot ON timetable(school, year, term, teacher, day, start);
CREATE UNIQUE INDEX uq_timetable_class_slot ON timetable(school, year, term, grade, stream, day, start);

CREATE TABLE lessons (
    school text not null,
    year integer not null,
    term smallint not null,
    grade smallint not null,
    stream smallint not null,
    date integer not null,
    subject smallint not null,
    teacher text not null,
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    PRIMARY KEY (school, year, term, grade, stream, date, subject, teacher),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (school, year, term) references terms(school, year, term) ON DELETE CASCADE,
    foreign key (school, teacher) references teachers(school, user) ON DELETE CASCADE,
    foreign key (school, year, term, grade, stream, subject) references subjects(school, year, term, grade, stream, subject) ON DELETE RESTRICT
);

CREATE TABLE exams (
    id text not null primary key,
    school text not null,
    year integer not null,
    term smallint not null,
    grade smallint not null,
    stream smallint, -- null means the exam spans all streams of the grade
    personalized boolean not null default false,
    type smallint not null, -- either Exam = 0, Assignment = 1, Assessment = 2.
    start integer not null,
    end integer not null,
    teacher text not null,
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    CHECK (start < end),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (school, year, term) references terms(school, year, term) ON DELETE CASCADE,
    foreign key (school, teacher) references teachers(school, user) ON DELETE CASCADE
);

CREATE TABLE papers (
    school text not null,
    exam text not null,
    subject smallint not null,
    paper smallint, -- this for indicating paper number like paper 1, 2 and 3 for subjects that need it.
    invigilator text not null,
    start bigint not null,
    end bigint not null,
    status smallint not null default 0, -- Pending = 0, Progress = 1, Done = 2, Marked = 3.
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    CHECK (start < end),
    primary key (school, exam, subject, paper),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (exam) references exams(id) ON DELETE CASCADE,
    foreign key (school, invigilator) references teachers(school, user) ON DELETE CASCADE
);
-- Enforces at most one null-paper row per (school, exam, subject).
-- Numbered papers for the same subject can coexist (e.g. English: composition + essay).
CREATE UNIQUE INDEX papers_subject_null_idx ON papers(school, exam, subject) WHERE paper IS NULL;

-- grades of exams
CREATE TABLE grades (
    school text not null,
    exam text not null,
    student integer not null,
    subject smallint not null,
    paper smallint, -- null means subject-level total; non-null matches a specific papers.paper row.
    score real not null,
    total integer not null,
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    CHECK (total > 0 AND score >= 0 AND score <= total),
    primary key (school, exam, student, subject, paper),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (exam) references exams(id) ON DELETE CASCADE,
    foreign key (school, student) references students(school, adm) ON DELETE CASCADE,
    -- Only enforced when paper IS NOT NULL (SQLite skips FK checks when any FK column is NULL).
    -- A null paper grade is a subject-level aggregate not tied to a specific papers row.
    foreign key (school, exam, subject, paper) references papers(school, exam, subject, paper) ON DELETE CASCADE
);

CREATE TABLE fees (
    id text not null primary key,
    school text not null,
    year integer not null,
    term smallint not null,
    grade smallint not null,
    title text not null,
    description text not null,
    amount real not null,
    mandatory boolean not null default true,
    due bigint not null,
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    CHECK (amount > 0),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (school, year, term) references terms(school, year, term) ON DELETE CASCADE
);

CREATE TABLE invoices (
    id text not null primary key,
    school text not null,
    year integer not null,
    term smallint not null,
    fee text,
    description text,
    student integer not null,
    amount real not null,
    status smallint not null default 0, -- Pending = 0, Partial = 1, Paid = 2, Overdue = 3, Cancelled = 4
    due bigint,  -- null inherits due date from the linked fee; required when fee is null
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    CHECK (amount > 0),
    CHECK (fee IS NOT NULL OR (description IS NOT NULL AND due IS NOT NULL)),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (fee) references fees(id) ON DELETE CASCADE,
    foreign key (school, student) references students(school, adm) ON DELETE CASCADE,
    foreign key (school, year, term) references terms(school, year, term) ON DELETE CASCADE
);

CREATE TABLE payments (
    id text not null primary key,
    invoice text,
    school text,
    student integer,
    amount real not null,
    method smallint not null default 0, -- Cash = 0, Cheque = 1, Mpesa = 2, Bank = 3
    reference text,
    recorder text, -- The member who recorded this payment for accountability in the case of cash. else it would be system.
    date integer, -- days since epoch; required for direct payments (when invoice is null) as a reference date
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    CHECK (amount > 0),
    CHECK (invoice IS NOT NULL OR (school IS NOT NULL AND student IS NOT NULL AND date IS NOT NULL)),
    foreign key (invoice) references invoices(id) ON DELETE CASCADE,
    foreign key (recorder) references users(id) ON DELETE SET NULL,
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (school, student) references students(school, adm) ON DELETE CASCADE
);

CREATE TABLE announcements (
    id text not null primary key,
    school text not null,
    title text not null,
    content text not null,
    grade smallint,
    stream smallint,
    audience integer not null, -- bitmask: bit 0 = Students (1), bit 1 = Parents (2), bit 2 = Teachers (4), bit 3 = Staff (8). 0 = All (no filter).
    author text, -- nullable: set to null when the author user is deleted; announcement is preserved.
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (author) references users(id) ON DELETE SET NULL
);

-- How good a student is on a particular topic of a particular subject of a particular grade
CREATE TABLE mastery (
    school text not null,
    student integer not null,
    grade smallint not null,
    subject smallint not null,
    topic smallint not null,
    score real not null,
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    primary key (school, student, grade, subject, topic),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (school, student) references students(school, adm) ON DELETE CASCADE
);

-- How many tokens has each student used in a term against his allocated amount.
CREATE TABLE aiusage (
    school text not null,
    student integer not null,
    year integer not null,
    term smallint not null,
    allocated integer not null,
    used integer not null,
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    primary key (school, student, year, term),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (school, student) references students(school, adm) ON DELETE CASCADE,
    foreign key (school, year, term) references terms(school, year, term) ON DELETE CASCADE
);

CREATE TABLE settings (
    school text not null primary key,
    data text not null, -- This would be a json value that holds everything on settings about the school
    mpesa text, -- json value for holding mpesa configuration details from the above commented table.
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    foreign key (school) references schools(id) ON DELETE CASCADE
);

CREATE TABLE roles (
    id text not null primary key,
    school text,
    name text not null,
    description text,
    permissions text not null, -- json map of permissions.
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    foreign key (school) references schools(id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX roles_school_name_idx ON roles(school, name) WHERE school IS NOT NULL;
CREATE UNIQUE INDEX roles_system_name_idx ON roles(name) WHERE school IS NULL;

-- This table is for associating/assigning roles to users
CREATE TABLE scopes (
    school text,
    user text not null,
    role text not null,
    created bigint not null default (unixepoch('now')),
    primary key (school, user, role),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (user) references users(id) ON DELETE CASCADE,
    foreign key (role) references roles(id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX scopes_system_idx ON scopes(user, role) WHERE school IS NULL;

CREATE TABLE plans (
    id text not null primary key,
    name text not null,
    description text,
    amount real not null,
    levels integer not null, -- bitmask of grade levels whose students can subscribe to this plan (e.g. pre-primary, junior-primary, senior-primary, etc.).
    status smallint not null default 0, -- either Pending = 0, Active = 1, Suspended = 2, Deleted = 3.
    features text, -- json based features map.
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    CHECK (amount >= 0)
);

CREATE TABLE subscriptions (
    school text not null,
    plan text not null,
    year integer not null,
    term smallint not null,
    student integer not null,
    invoice text, -- the invoice generated for this subscription; null until invoiced
    discount real not null default 0, -- discount in percentage.
    status smallint not null default 0, -- either Pending = 0, Active = 1, Cancelled = 2, Deleted = 3.
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    CHECK (discount >= 0 AND discount <= 100),
    primary key (school, plan, year, term, student),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (plan) references plans(id) ON DELETE CASCADE,
    foreign key (school, student) references students(school, adm) ON DELETE CASCADE,
    foreign key (school, year, term) references terms(school, year, term) ON DELETE CASCADE,
    foreign key (invoice) references invoices(id) ON DELETE SET NULL
);

CREATE TABLE discounts (
    school text not null,
    plan text not null,
    year integer not null,
    term smallint not null,
    grade smallint not null,
    amount real not null,
    unit smallint not null default 0, -- 0 = Percentage, 1 = Amount
    created bigint not null default (unixepoch('now')),
    updated bigint not null default (unixepoch('now')),
    CHECK (amount >= 0 AND (unit != 0 OR amount <= 100)),
    primary key (school, plan, year, term, grade),
    foreign key (school) references schools(id) ON DELETE CASCADE,
    foreign key (plan) references plans(id) ON DELETE CASCADE,
    foreign key (school, year, term) references terms(school, year, term) ON DELETE CASCADE
);

-- ============================================================
-- Triggers
-- ============================================================

-- Prevents mixing null-paper (subject-level) and numbered-paper grades for the same
-- student+subject within the same exam. Mirrors the papers_subject_null_idx logic at
-- the grades level so aggregations never double-count.
CREATE TRIGGER grades_paper_mix_check
    BEFORE INSERT ON grades
BEGIN
    SELECT RAISE(ABORT, 'cannot add a numbered-paper grade: a subject-level grade already exists for this student in this exam')
    WHERE NEW.paper IS NOT NULL
    AND EXISTS (
        SELECT 1 FROM grades
        WHERE school = NEW.school AND exam = NEW.exam AND student = NEW.student
          AND subject = NEW.subject AND paper IS NULL
    );
    SELECT RAISE(ABORT, 'cannot add a subject-level grade: numbered-paper grades already exist for this student in this exam')
    WHERE NEW.paper IS NULL
    AND EXISTS (
        SELECT 1 FROM grades
        WHERE school = NEW.school AND exam = NEW.exam AND student = NEW.student
          AND subject = NEW.subject AND paper IS NOT NULL
    );
END;

-- Ensures a grade can only be recorded for a student who is enrolled in the class the
-- exam belongs to. When the exam spans all streams (stream IS NULL), any stream enrollment
-- for that grade/term counts. The school pin on the JOIN prevents a cross-school exam id
-- from satisfying the check.
CREATE TRIGGER grades_enrollment_check
    BEFORE INSERT ON grades
BEGIN
    SELECT RAISE(ABORT, 'student is not enrolled in the class this exam belongs to')
    WHERE NOT EXISTS (
        SELECT 1 FROM enrollments
        INNER JOIN exams ON exams.id = NEW.exam AND exams.school = NEW.school
        WHERE enrollments.school  = NEW.school
          AND enrollments.student = NEW.student
          AND enrollments.year    = exams.year
          AND enrollments.term    = exams.term
          AND enrollments.grade   = exams.grade
          AND (exams.stream IS NULL OR enrollments.stream = exams.stream)
    );
END;

-- Prevents mixing all-stream and stream-specific exams of the same type for the same
-- grade/term. Without this, a Grade 8 all-stream exam and a Grade 8 Stream A exam of
-- the same type could coexist, making it ambiguous which exam a student should sit.
CREATE TRIGGER exams_stream_consistency_check
    BEFORE INSERT ON exams
BEGIN
    SELECT RAISE(ABORT, 'an all-stream exam of this type already exists for this grade/term; cannot add a stream-specific exam')
    WHERE NEW.stream IS NOT NULL
    AND EXISTS (
        SELECT 1 FROM exams
        WHERE school = NEW.school AND year = NEW.year AND term = NEW.term
          AND grade = NEW.grade AND type = NEW.type AND stream IS NULL
    );
    SELECT RAISE(ABORT, 'stream-specific exams of this type already exist for this grade/term; cannot add an all-stream exam')
    WHERE NEW.stream IS NULL
    AND EXISTS (
        SELECT 1 FROM exams
        WHERE school = NEW.school AND year = NEW.year AND term = NEW.term
          AND grade = NEW.grade AND type = NEW.type AND stream IS NOT NULL
    );
END;

-- Prevents UPDATE from introducing a paper-mix violation on grades.
CREATE TRIGGER grades_paper_mix_check_update
    BEFORE UPDATE OF paper ON grades
BEGIN
    SELECT RAISE(ABORT, 'cannot update to a numbered-paper grade: a subject-level grade already exists for this student in this exam')
    WHERE NEW.paper IS NOT NULL
    AND EXISTS (
        SELECT 1 FROM grades
        WHERE school = NEW.school AND exam = NEW.exam AND student = NEW.student
          AND subject = NEW.subject AND paper IS NULL
          AND rowid IS NOT OLD.rowid
    );
    SELECT RAISE(ABORT, 'cannot update to a subject-level grade: numbered-paper grades already exist for this student in this exam')
    WHERE NEW.paper IS NULL
    AND EXISTS (
        SELECT 1 FROM grades
        WHERE school = NEW.school AND exam = NEW.exam AND student = NEW.student
          AND subject = NEW.subject AND paper IS NOT NULL
          AND rowid IS NOT OLD.rowid
    );
END;

-- Prevents UPDATE from moving a grade to an exam the student is not enrolled in.
CREATE TRIGGER grades_enrollment_check_update
    BEFORE UPDATE OF exam, student, school ON grades
BEGIN
    SELECT RAISE(ABORT, 'student is not enrolled in the class this exam belongs to')
    WHERE NOT EXISTS (
        SELECT 1 FROM enrollments
        INNER JOIN exams ON exams.id = NEW.exam AND exams.school = NEW.school
        WHERE enrollments.school  = NEW.school
          AND enrollments.student = NEW.student
          AND enrollments.year    = exams.year
          AND enrollments.term    = exams.term
          AND enrollments.grade   = exams.grade
          AND (exams.stream IS NULL OR enrollments.stream = exams.stream)
    );
END;

-- Ensures the invoice linked to a subscription (on INSERT) belongs to the same student
-- and school, preventing a cross-student invoice from being attached.
CREATE TRIGGER subscriptions_invoice_check
    BEFORE INSERT ON subscriptions
BEGIN
    SELECT RAISE(ABORT, 'invoice does not belong to this student or school')
    WHERE NEW.invoice IS NOT NULL
    AND NOT EXISTS (
        SELECT 1 FROM invoices
        WHERE id = NEW.invoice
          AND school = NEW.school
          AND student = NEW.student
    );
END;

-- Same check on UPDATE so reassigning the invoice column is equally validated.
CREATE TRIGGER subscriptions_invoice_check_update
    BEFORE UPDATE OF invoice ON subscriptions
BEGIN
    SELECT RAISE(ABORT, 'invoice does not belong to this student or school')
    WHERE NEW.invoice IS NOT NULL
    AND NOT EXISTS (
        SELECT 1 FROM invoices
        WHERE id = NEW.invoice
          AND school = NEW.school
          AND student = NEW.student
    );
END;

-- Prevents inserting a term whose date range overlaps any existing term for the same school.
CREATE TRIGGER terms_no_overlap
    BEFORE INSERT ON terms
BEGIN
    SELECT RAISE(ABORT, 'term dates overlap with an existing term for this school')
    WHERE EXISTS (
        SELECT 1 FROM terms
        WHERE school = NEW.school
          AND start < NEW.end
          AND end   > NEW.start
    );
END;

-- Same overlap check when a term's dates are updated.
CREATE TRIGGER terms_no_overlap_update
    BEFORE UPDATE OF start, end ON terms
BEGIN
    SELECT RAISE(ABORT, 'term dates overlap with an existing term for this school')
    WHERE EXISTS (
        SELECT 1 FROM terms
        WHERE school = NEW.school
          AND start < NEW.end
          AND end   > NEW.start
          AND NOT (year = OLD.year AND term = OLD.term)
    );
END;

-- Ensures a paper is not scheduled outside its parent exam's date window.
-- papers.start/end are bigint (seconds since epoch); exams.start/end are integer (days since epoch).
CREATE TRIGGER papers_within_exam_range
    BEFORE INSERT ON papers
BEGIN
    SELECT RAISE(ABORT, 'paper schedule falls outside the exam date range')
    WHERE NOT EXISTS (
        SELECT 1 FROM exams
        WHERE id = NEW.exam
          AND NEW.start >= start * 86400
          AND NEW.end   <= (end + 1) * 86400
    );
END;

CREATE TRIGGER papers_within_exam_range_update
    BEFORE UPDATE OF start, end, exam ON papers
BEGIN
    SELECT RAISE(ABORT, 'paper schedule falls outside the exam date range')
    WHERE NOT EXISTS (
        SELECT 1 FROM exams
        WHERE id = NEW.exam
          AND NEW.start >= start * 86400
          AND NEW.end   <= (end + 1) * 86400
    );
END;

-- Ensures attendance is only recorded on dates that fall within the term.
-- attendance.date is integer (days since epoch); terms.start/end are bigint (seconds since epoch).
CREATE TRIGGER attendance_within_term
    BEFORE INSERT ON attendance
BEGIN
    SELECT RAISE(ABORT, 'attendance date falls outside the term date range')
    WHERE NOT EXISTS (
        SELECT 1 FROM terms
        WHERE school = NEW.school AND year = NEW.year AND term = NEW.term
          AND NEW.date >= start / 86400
          AND NEW.date <= end   / 86400
    );
END;

CREATE TRIGGER attendance_within_term_update
    BEFORE UPDATE OF date ON attendance
BEGIN
    SELECT RAISE(ABORT, 'attendance date falls outside the term date range')
    WHERE NOT EXISTS (
        SELECT 1 FROM terms
        WHERE school = NEW.school AND year = NEW.year AND term = NEW.term
          AND NEW.date >= start / 86400
          AND NEW.date <= end   / 86400
    );
END;

-- Ensures a lesson is only recorded on a date that falls within the term.
CREATE TRIGGER lessons_within_term
    BEFORE INSERT ON lessons
BEGIN
    SELECT RAISE(ABORT, 'lesson date falls outside the term date range')
    WHERE NOT EXISTS (
        SELECT 1 FROM terms
        WHERE school = NEW.school AND year = NEW.year AND term = NEW.term
          AND NEW.date >= start / 86400
          AND NEW.date <= end   / 86400
    );
END;

CREATE TRIGGER lessons_within_term_update
    BEFORE UPDATE OF date ON lessons
BEGIN
    SELECT RAISE(ABORT, 'lesson date falls outside the term date range')
    WHERE NOT EXISTS (
        SELECT 1 FROM terms
        WHERE school = NEW.school AND year = NEW.year AND term = NEW.term
          AND NEW.date >= start / 86400
          AND NEW.date <= end   / 86400
    );
END;

-- When a department is deleted, only nullify the department column in teachers and staff
-- (not school). A composite FK with ON DELETE SET NULL would wipe both columns, so we
-- use NO ACTION on the FK and handle the cascade here instead.
CREATE TRIGGER dept_delete_clear_teachers
    AFTER DELETE ON departments
BEGIN
    UPDATE teachers SET department = NULL
    WHERE school = OLD.school AND department = OLD.name;
END;

CREATE TRIGGER dept_delete_clear_staff
    AFTER DELETE ON departments
BEGIN
    UPDATE staff SET department = NULL
    WHERE school = OLD.school AND department = OLD.name;
END;

-- ============================================================
-- Performance indexes (non-unique, for retrieval speed)
-- ============================================================

-- users: lookup by email (login/search), filter by status/level
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_status ON users(status);

-- schools: filter by status, geographic queries by county, subdomain routing by domain
CREATE INDEX idx_schools_status ON schools(status);
CREATE INDEX idx_schools_county ON schools(county);
CREATE UNIQUE INDEX idx_schools_domain ON schools(domain) WHERE domain IS NOT NULL;

-- owners: find all schools owned by a user (user is not the leading PK column)
CREATE INDEX idx_owners_user ON owners(user);

-- students: filter active/expelled/etc students per school; name search within a school
CREATE INDEX idx_students_school_status ON students(school, status);
CREATE INDEX idx_students_school_name ON students(school, name);

-- guardians: find all guardians of a student (student is not the leading PK column)
CREATE INDEX idx_guardians_school_student ON guardians(school, student);

-- teachers: list teachers by department; filter by status within a school
CREATE INDEX idx_teachers_school_department ON teachers(school, department);
CREATE INDEX idx_teachers_school_status ON teachers(school, status);

-- staff: filter by school, department and status
CREATE INDEX idx_staff_school ON staff(school);
CREATE INDEX idx_staff_school_department ON staff(school, department);
CREATE INDEX idx_staff_school_status ON staff(school, status);

-- class_teachers: find all class assignments for a teacher; quickly locate the active teacher per class
CREATE INDEX idx_class_teachers_school_teacher ON class_teachers(school, teacher);
CREATE UNIQUE INDEX uq_class_teachers_active ON class_teachers(school, year, term, grade, stream) WHERE end IS NULL;

-- enrollments: find all classes a student has been enrolled in (student is 6th in PK)
CREATE INDEX idx_enrollments_school_student ON enrollments(school, student);

-- subjects: find all subjects taught by a teacher (teacher is not in PK)
CREATE INDEX idx_subjects_school_teacher ON subjects(school, teacher);

-- attendance: pull a student's attendance record for a term (student is 6th in PK)
CREATE INDEX idx_attendance_school_term_student ON attendance(school, year, term, student);

-- timetable: look up a teacher's full timetable (teacher is not in PK)
CREATE INDEX idx_timetable_school_teacher ON timetable(school, teacher);

-- lessons: find lessons delivered by a teacher; browse lessons by date within a term
CREATE INDEX idx_lessons_school_teacher ON lessons(school, teacher);
CREATE INDEX idx_lessons_school_term_date ON lessons(school, year, term, date);

-- exams: list all exams for a class in a term (PK is a surrogate id); find exams by teacher
CREATE INDEX idx_exams_school_term_class ON exams(school, year, term, grade, stream);
CREATE INDEX idx_exams_school_teacher ON exams(school, teacher);
-- Prevents two all-stream exams of the same type for the same grade/term
CREATE UNIQUE INDEX uq_exams_allstream_type ON exams(school, year, term, grade, type) WHERE stream IS NULL;
-- Prevents two stream-specific exams of the same type for the same class
CREATE UNIQUE INDEX uq_exams_stream_type ON exams(school, year, term, grade, stream, type) WHERE stream IS NOT NULL;

-- papers: list all papers for an exam filtered by status (school+exam covered by PK prefix)
CREATE INDEX idx_papers_school_exam_status ON papers(school, exam, status);

-- grades: pull all grades for a student across exams (student is 3rd in PK)
CREATE INDEX idx_grades_school_student ON grades(school, student);

-- fees: list fees applicable to a class in a given term (PK is a surrogate id)
CREATE INDEX idx_fees_school_term_grade ON fees(school, year, term, grade);

-- invoices: student statement (PK is surrogate id); term-level overview; filter by payment status
CREATE INDEX idx_invoices_school_student ON invoices(school, student);
CREATE INDEX idx_invoices_school_term ON invoices(school, year, term);
CREATE INDEX idx_invoices_school_status ON invoices(school, status);

-- payments: find all payments against an invoice; pull payment history for a student
CREATE INDEX idx_payments_invoice ON payments(invoice);
CREATE INDEX idx_payments_school_student ON payments(school, student);
-- Direct (invoice-less) payments queried by date for cash ledger reporting
CREATE INDEX idx_payments_direct_date ON payments(school, student, date) WHERE invoice IS NULL;

-- announcements: list announcements for a school; filter class-targeted announcements
CREATE INDEX idx_announcements_school ON announcements(school);
CREATE INDEX idx_announcements_school_grade ON announcements(school, grade);

-- scopes: find all users assigned a specific role (role is not the leading PK column)
CREATE INDEX idx_scopes_school_role ON scopes(school, role);
CREATE INDEX idx_scopes_role ON scopes(role);

-- plans: filter by status (e.g. show only Active plans)
CREATE INDEX idx_plans_status ON plans(status);

-- subscriptions: find a student's subscriptions (student is 5th in PK); list per term
CREATE INDEX idx_subscriptions_school_student ON subscriptions(school, student);
CREATE INDEX idx_subscriptions_school_term ON subscriptions(school, year, term);

-- discounts: look up discounts by class/grade without knowing the plan upfront
CREATE INDEX idx_discounts_school_term_grade ON discounts(school, year, term, grade);
