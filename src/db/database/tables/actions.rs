#![allow(dead_code)]

use super::delete;
use super::insert;
use super::rows::*;
use super::update;
use crate::db::changelog::{LOG, Record};
use crate::proto::services::sync::*;
use crate::types::error::{Error, Result};
use crate::types::id::Id;
use crate::types::role::{Action, Resource};
use diesel::RunQueryDsl;
use diesel::SqliteConnection as Conn;
use diesel::sql_query;
use diesel::sql_types::Text;
use prost::Message;

#[derive(diesel::QueryableByName)]
struct FkCheckRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    cnt: i64,
}

/// SyncAction integer values — must match the client's SyncAction enum.
pub mod sync_action {
    pub const CREATE_SCHOOL: i32 = 0;
    pub const UPDATE_SCHOOL: i32 = 1;
    pub const DELETE_SCHOOL: i32 = 2;
    pub const CREATE_TEACHER: i32 = 3;
    pub const UPDATE_TEACHER: i32 = 4;
    pub const DELETE_TEACHER: i32 = 5;
    pub const CREATE_STAFF: i32 = 6;
    pub const UPDATE_STAFF: i32 = 7;
    pub const DELETE_STAFF: i32 = 8;
    pub const CREATE_OWNER: i32 = 9;
    pub const DELETE_OWNER: i32 = 10;
    pub const CREATE_STUDENT: i32 = 11;
    pub const UPDATE_STUDENT: i32 = 12;
    pub const DELETE_STUDENT: i32 = 13;
    pub const ENROLL_STUDENT: i32 = 14;
    pub const UNENROLL_STUDENT: i32 = 15;
    pub const CREATE_GUARDIAN: i32 = 16;
    pub const UPDATE_GUARDIAN: i32 = 17;
    pub const DELETE_GUARDIAN: i32 = 18;
    pub const CREATE_DEPARTMENT: i32 = 19;
    pub const UPDATE_DEPARTMENT: i32 = 20;
    pub const DELETE_DEPARTMENT: i32 = 21;
    pub const CREATE_TERM: i32 = 22;
    pub const UPDATE_TERM: i32 = 23;
    pub const DELETE_TERM: i32 = 24;
    pub const ASSIGN_CLASS_TEACHER: i32 = 25;
    pub const UNASSIGN_CLASS_TEACHER: i32 = 26;
    pub const ASSIGN_SUBJECT: i32 = 27;
    pub const UNASSIGN_SUBJECT: i32 = 28;
    pub const CREATE_TIMETABLE_ENTRY: i32 = 29;
    pub const UPDATE_TIMETABLE_ENTRY: i32 = 30;
    pub const DELETE_TIMETABLE_ENTRY: i32 = 31;
    pub const MARK_ATTENDANCE: i32 = 32;
    pub const DELETE_ATTENDANCE: i32 = 33;
    pub const CREATE_LESSON: i32 = 34;
    pub const DELETE_LESSON: i32 = 35;
    pub const CREATE_EXAM: i32 = 36;
    pub const UPDATE_EXAM: i32 = 37;
    pub const DELETE_EXAM: i32 = 38;
    pub const CREATE_PAPER: i32 = 39;
    pub const UPDATE_PAPER: i32 = 40;
    pub const DELETE_PAPER: i32 = 41;
    pub const MARK_GRADES: i32 = 42;
    pub const UPDATE_GRADE: i32 = 43;
    pub const DELETE_GRADE: i32 = 44;
    pub const UPDATE_MASTERY: i32 = 45;
    pub const CREATE_FEE: i32 = 46;
    pub const UPDATE_FEE: i32 = 47;
    pub const DELETE_FEE: i32 = 48;
    pub const CREATE_INVOICE: i32 = 49;
    pub const UPDATE_INVOICE: i32 = 50;
    pub const DELETE_INVOICE: i32 = 51;
    pub const CREATE_PAYMENT: i32 = 52;
    pub const UPDATE_PAYMENT: i32 = 53;
    pub const DELETE_PAYMENT: i32 = 54;
    pub const APPROVE_PAYMENT: i32 = 55;
    pub const CREATE_ANNOUNCEMENT: i32 = 56;
    pub const UPDATE_ANNOUNCEMENT: i32 = 57;
    pub const DELETE_ANNOUNCEMENT: i32 = 58;
    pub const CREATE_ROLE: i32 = 59;
    pub const UPDATE_ROLE: i32 = 60;
    pub const DELETE_ROLE: i32 = 61;
    pub const ASSIGN_ROLE: i32 = 62;
    pub const UNASSIGN_ROLE: i32 = 63;
    pub const UPDATE_USER: i32 = 64;
    pub const DELETE_USER: i32 = 65;
    pub const UPDATE_SETTINGS: i32 = 66;
    pub const CREATE_PLAN: i32 = 67;
    pub const UPDATE_PLAN: i32 = 68;
    pub const DELETE_PLAN: i32 = 69;
    pub const UPDATE_AI_USAGE: i32 = 70;
    pub const CREATE_SUBSCRIPTION: i32 = 71;
    pub const UPDATE_SUBSCRIPTION: i32 = 72;
    pub const DELETE_SUBSCRIPTION: i32 = 73;
    pub const CREATE_DISCOUNT: i32 = 74;
    pub const UPDATE_DISCOUNT: i32 = 75;
    pub const DELETE_DISCOUNT: i32 = 76;
    pub const CREATE_SUBJECT: i32 = 77;
    pub const UPDATE_SUBJECT: i32 = 78;
    pub const DELETE_SUBJECT: i32 = 79;
    pub const CREATE_TOPIC: i32 = 80;
    pub const UPDATE_TOPIC: i32 = 81;
    pub const DELETE_TOPIC: i32 = 82;
    pub const CREATE_STREAM: i32 = 83;
    pub const UPDATE_STREAM: i32 = 84;
    pub const DELETE_STREAM: i32 = 85;
    pub const CREATE_MPESA: i32 = 86;
    pub const UPDATE_MPESA: i32 = 87;
    pub const DELETE_MPESA: i32 = 88;
    // 89, 90: reserved (removed exam_grade actions)
    pub const UPLOAD_SCHEME: i32 = 91;
    pub const DELETE_SCHEME: i32 = 92;
    pub const UPLOAD_ANSWER_SHEET: i32 = 93;
    pub const DELETE_ANSWER_SHEET: i32 = 94;
}

/// Result of executing a single action. Contains the rows to return to the
/// client and any presigned file URLs.
pub struct ActionResult {
    pub rows: Vec<ActionRow>,
    pub file_urls: Vec<FileUrl>,
}

impl ActionResult {
    pub fn empty() -> Self {
        Self {
            rows: Vec::new(),
            file_urls: Vec::new(),
        }
    }

    pub fn with_rows(rows: Vec<ActionRow>) -> Self {
        Self {
            rows,
            file_urls: Vec::new(),
        }
    }

    pub fn with_rows_and_urls(rows: Vec<ActionRow>, file_urls: Vec<FileUrl>) -> Self {
        Self { rows, file_urls }
    }
}

/// Maps a SyncAction integer to the `(Resource, Action)` pair required for
/// authorization. This tells the authorization layer *what* permission the
/// caller needs before the action can be executed.
pub fn action_permission(action_id: i32) -> Result<(Resource, Action)> {
    use sync_action::*;
    match action_id {
        // Schools
        CREATE_SCHOOL => Ok((Resource::Schools, Action::Create)),
        UPDATE_SCHOOL => Ok((Resource::Schools, Action::Update)),
        DELETE_SCHOOL => Ok((Resource::Schools, Action::Delete)),

        // Teachers
        CREATE_TEACHER => Ok((Resource::Teachers, Action::Create)),
        UPDATE_TEACHER => Ok((Resource::Teachers, Action::Update)),
        DELETE_TEACHER => Ok((Resource::Teachers, Action::Delete)),

        // Staff
        CREATE_STAFF => Ok((Resource::Staff, Action::Create)),
        UPDATE_STAFF => Ok((Resource::Staff, Action::Update)),
        DELETE_STAFF => Ok((Resource::Staff, Action::Delete)),

        // Owners
        CREATE_OWNER => Ok((Resource::Owners, Action::Create)),
        DELETE_OWNER => Ok((Resource::Owners, Action::Delete)),

        // Students
        CREATE_STUDENT => Ok((Resource::Students, Action::Create)),
        UPDATE_STUDENT => Ok((Resource::Students, Action::Update)),
        DELETE_STUDENT => Ok((Resource::Students, Action::Delete)),
        ENROLL_STUDENT => Ok((Resource::Students, Action::Assign)),
        UNENROLL_STUDENT => Ok((Resource::Students, Action::Unassign)),

        // Guardians
        CREATE_GUARDIAN => Ok((Resource::Students, Action::Create)),
        UPDATE_GUARDIAN => Ok((Resource::Students, Action::Update)),
        DELETE_GUARDIAN => Ok((Resource::Students, Action::Delete)),

        // Departments
        CREATE_DEPARTMENT => Ok((Resource::Departments, Action::Create)),
        UPDATE_DEPARTMENT => Ok((Resource::Departments, Action::Update)),
        DELETE_DEPARTMENT => Ok((Resource::Departments, Action::Delete)),

        // Terms
        CREATE_TERM => Ok((Resource::Schools, Action::Create)),
        UPDATE_TERM => Ok((Resource::Schools, Action::Update)),
        DELETE_TERM => Ok((Resource::Schools, Action::Delete)),

        // Class teachers
        ASSIGN_CLASS_TEACHER => Ok((Resource::Classes, Action::Assign)),
        UNASSIGN_CLASS_TEACHER => Ok((Resource::Classes, Action::Unassign)),

        // Subjects
        ASSIGN_SUBJECT => Ok((Resource::Classes, Action::Assign)),
        UNASSIGN_SUBJECT => Ok((Resource::Classes, Action::Unassign)),

        // Timetable
        CREATE_TIMETABLE_ENTRY => Ok((Resource::Classes, Action::Create)),
        UPDATE_TIMETABLE_ENTRY => Ok((Resource::Classes, Action::Update)),
        DELETE_TIMETABLE_ENTRY => Ok((Resource::Classes, Action::Delete)),

        // Attendance
        MARK_ATTENDANCE => Ok((Resource::Attendance, Action::Mark)),
        DELETE_ATTENDANCE => Ok((Resource::Attendance, Action::Delete)),

        // Lessons
        CREATE_LESSON => Ok((Resource::Lessons, Action::Create)),
        DELETE_LESSON => Ok((Resource::Lessons, Action::Delete)),

        // Exams
        CREATE_EXAM => Ok((Resource::Exams, Action::Create)),
        UPDATE_EXAM => Ok((Resource::Exams, Action::Update)),
        DELETE_EXAM => Ok((Resource::Exams, Action::Delete)),

        // Papers
        CREATE_PAPER => Ok((Resource::Exams, Action::Create)),
        UPDATE_PAPER => Ok((Resource::Exams, Action::Update)),
        DELETE_PAPER => Ok((Resource::Exams, Action::Delete)),

        // Grades
        MARK_GRADES => Ok((Resource::Grades, Action::Mark)),
        UPDATE_GRADE => Ok((Resource::Grades, Action::Update)),
        DELETE_GRADE => Ok((Resource::Grades, Action::Delete)),

        // Mastery
        UPDATE_MASTERY => Ok((Resource::Grades, Action::Mark)),

        // Fees
        CREATE_FEE => Ok((Resource::Fees, Action::Create)),
        UPDATE_FEE => Ok((Resource::Fees, Action::Update)),
        DELETE_FEE => Ok((Resource::Fees, Action::Delete)),

        // Invoices
        CREATE_INVOICE => Ok((Resource::Fees, Action::Create)),
        UPDATE_INVOICE => Ok((Resource::Fees, Action::Update)),
        DELETE_INVOICE => Ok((Resource::Fees, Action::Delete)),

        // Payments
        CREATE_PAYMENT => Ok((Resource::Payments, Action::Create)),
        UPDATE_PAYMENT => Ok((Resource::Payments, Action::Update)),
        DELETE_PAYMENT => Ok((Resource::Payments, Action::Delete)),
        APPROVE_PAYMENT => Ok((Resource::Payments, Action::Approve)),

        // Announcements
        CREATE_ANNOUNCEMENT => Ok((Resource::Announcements, Action::Create)),
        UPDATE_ANNOUNCEMENT => Ok((Resource::Announcements, Action::Update)),
        DELETE_ANNOUNCEMENT => Ok((Resource::Announcements, Action::Delete)),

        // Roles
        CREATE_ROLE => Ok((Resource::Roles, Action::Create)),
        UPDATE_ROLE => Ok((Resource::Roles, Action::Update)),
        DELETE_ROLE => Ok((Resource::Roles, Action::Delete)),
        ASSIGN_ROLE => Ok((Resource::Roles, Action::Assign)),
        UNASSIGN_ROLE => Ok((Resource::Roles, Action::Unassign)),

        // Users
        UPDATE_USER => Ok((Resource::Users, Action::Update)),
        DELETE_USER => Ok((Resource::Users, Action::Delete)),

        // Settings
        UPDATE_SETTINGS => Ok((Resource::Schools, Action::Update)),

        // Plans
        CREATE_PLAN => Ok((Resource::Plans, Action::Create)),
        UPDATE_PLAN => Ok((Resource::Plans, Action::Update)),
        DELETE_PLAN => Ok((Resource::Plans, Action::Delete)),

        // AI usage
        UPDATE_AI_USAGE => Ok((Resource::AI, Action::Update)),

        // Subscriptions
        CREATE_SUBSCRIPTION => Ok((Resource::Plans, Action::Create)),
        UPDATE_SUBSCRIPTION => Ok((Resource::Plans, Action::Update)),
        DELETE_SUBSCRIPTION => Ok((Resource::Plans, Action::Delete)),

        // Discounts
        CREATE_DISCOUNT => Ok((Resource::Plans, Action::Create)),
        UPDATE_DISCOUNT => Ok((Resource::Plans, Action::Update)),
        DELETE_DISCOUNT => Ok((Resource::Plans, Action::Delete)),

        // Subjects (global catalog)
        CREATE_SUBJECT => Ok((Resource::Subjects, Action::Create)),
        UPDATE_SUBJECT => Ok((Resource::Subjects, Action::Update)),
        DELETE_SUBJECT => Ok((Resource::Subjects, Action::Delete)),

        // Topics (global catalog)
        CREATE_TOPIC => Ok((Resource::Subjects, Action::Create)),
        UPDATE_TOPIC => Ok((Resource::Subjects, Action::Update)),
        DELETE_TOPIC => Ok((Resource::Subjects, Action::Delete)),

        // Streams
        CREATE_STREAM => Ok((Resource::Schools, Action::Create)),
        UPDATE_STREAM => Ok((Resource::Schools, Action::Update)),
        DELETE_STREAM => Ok((Resource::Schools, Action::Delete)),

        // Mpesa
        CREATE_MPESA => Ok((Resource::Schools, Action::Create)),
        UPDATE_MPESA => Ok((Resource::Schools, Action::Update)),
        DELETE_MPESA => Ok((Resource::Schools, Action::Delete)),

        // File sync: scheme & answer pages
        UPLOAD_SCHEME => Ok((Resource::Exams, Action::Update)),
        DELETE_SCHEME => Ok((Resource::Exams, Action::Delete)),
        UPLOAD_ANSWER_SHEET => Ok((Resource::Grades, Action::Mark)),
        DELETE_ANSWER_SHEET => Ok((Resource::Grades, Action::Delete)),

        // 89, 90: reserved (removed exam_grade actions)
        _ => {
            tracing::error!("action_permission: unknown action {action_id}");
            Err(Error::Internal)
        }
    }
}

/// Decode a protobuf payload, returning `Error::Internal` on failure.
fn decode<T: Message + Default>(payload: &[u8]) -> Result<T> {
    T::decode(payload).map_err(|e| {
        tracing::error!("failed to decode payload: {e}");
        Error::Internal
    })
}

// ---------------------------------------------------------------------------
// Table number constants (must match LogTable / InsertData oneof field numbers)
// ---------------------------------------------------------------------------

const TBL_USERS: i32 = 1;
const TBL_SCHOOLS: i32 = 2;
const TBL_OWNERS: i32 = 3;
const TBL_STUDENTS: i32 = 4;
const TBL_GUARDIANS: i32 = 5;
const TBL_DEPARTMENTS: i32 = 6;
const TBL_TEACHERS: i32 = 7;
const TBL_STAFF: i32 = 8;
const TBL_TERMS: i32 = 9;
const TBL_CLASS_TEACHERS: i32 = 10;
const TBL_ENROLLMENTS: i32 = 11;
const TBL_SUBJECTS: i32 = 12;
const TBL_ATTENDANCE: i32 = 13;
const TBL_TIMETABLE: i32 = 14;
const TBL_LESSONS: i32 = 15;
const TBL_EXAMS: i32 = 16;
const TBL_PAPERS: i32 = 17;
const TBL_GRADES: i32 = 18;
const TBL_FEES: i32 = 19;
const TBL_INVOICES: i32 = 20;
const TBL_PAYMENTS: i32 = 21;
const TBL_ANNOUNCEMENTS: i32 = 22;
const TBL_MASTERY: i32 = 23;
const TBL_AI_USAGE: i32 = 24;
const TBL_ROLES: i32 = 26;
const TBL_SCOPES: i32 = 27;
const TBL_PLANS: i32 = 28;
const TBL_SUBSCRIPTIONS: i32 = 29;
const TBL_DISCOUNTS: i32 = 30;
const TBL_SUBJECT_CATALOG: i32 = 31;
const TBL_TOPICS: i32 = 32;
const TBL_STREAMS: i32 = 33;
const TBL_MPESA: i32 = 34;
// TBL_EXAM_GRADES (35) removed — grade/stream moved to papers
const TBL_SCHEME_PAGES: i32 = 36;
const TBL_ANSWER_PAGES: i32 = 37;

// Changelog operation constants
const OP_INSERT: u8 = 0;
const OP_UPDATE: u8 = 1;
const OP_DELETE: u8 = 2;

// ---------------------------------------------------------------------------
// Single-row query helpers (fetch a row back after insert/update)
// ---------------------------------------------------------------------------

fn fetch_user(conn: &mut Conn, id: &str) -> Result<UserRow> {
    sql_query(
        "SELECT id, phone, email, name, level, status, created, updated FROM users WHERE id = ?",
    )
    .bind::<Text, _>(id)
    .load::<UserRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_user failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or(Error::UserNotFound)
}

fn fetch_school(conn: &mut Conn, id: &str) -> Result<SchoolRow> {
    sql_query("SELECT id, name, motto, phone, email, county, domain, established, status, created, updated FROM schools WHERE id = ?")
        .bind::<Text, _>(id)
        .load::<SchoolRow>(conn)
        .map_err(|e| { tracing::error!("fetch_school failed: {e}"); Error::Internal })?
        .into_iter()
        .next()
        .ok_or_else(|| { tracing::error!("school not found: {id}"); Error::Internal })
}

fn fetch_owner(conn: &mut Conn, school: &str, user: &str) -> Result<OwnerRow> {
    sql_query("SELECT school, user, created FROM owners WHERE school = ? AND user = ?")
        .bind::<Text, _>(school)
        .bind::<Text, _>(user)
        .load::<OwnerRow>(conn)
        .map_err(|e| {
            tracing::error!("fetch_owner failed: {e}");
            Error::Internal
        })?
        .into_iter()
        .next()
        .ok_or_else(|| {
            tracing::error!("owner not found: {school}|{user}");
            Error::Internal
        })
}

fn fetch_plan(conn: &mut Conn, id: &str) -> Result<PlanRow> {
    sql_query("SELECT id, name, description, amount, levels, status, features, created, updated FROM plans WHERE id = ?")
        .bind::<Text, _>(id)
        .load::<PlanRow>(conn)
        .map_err(|e| { tracing::error!("fetch_plan failed: {e}"); Error::Internal })?
        .into_iter()
        .next()
        .ok_or_else(|| { tracing::error!("plan not found: {id}"); Error::Internal })
}

fn fetch_user_by_phone(conn: &mut Conn, phone: &str) -> Result<Option<UserRow>> {
    let rows = sql_query(
        "SELECT id, phone, email, name, level, status, created, updated FROM users WHERE phone = ?",
    )
    .bind::<Text, _>(phone)
    .load::<UserRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_user_by_phone failed: {e}");
        Error::Internal
    })?;
    Ok(rows.into_iter().next())
}

fn fetch_teacher(conn: &mut Conn, school: &str, user: &str) -> Result<TeacherRow> {
    sql_query(
        "SELECT school, user, hired, role, department, status, created, updated \
         FROM teachers WHERE school = ? AND user = ?",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(user)
    .load::<TeacherRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_teacher failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("teacher not found: {school}|{user}");
        Error::Internal
    })
}

fn fetch_staff(conn: &mut Conn, school: &str, user: &str) -> Result<StaffRow> {
    sql_query(
        "SELECT school, user, idnumber, role, department, status, created, updated \
         FROM staff WHERE school = ? AND user = ?",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(user)
    .load::<StaffRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_staff failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("staff not found: {school}|{user}");
        Error::Internal
    })
}

fn fetch_guardian(conn: &mut Conn, school: &str, user: &str, student: i32) -> Result<GuardianRow> {
    sql_query(
        "SELECT school, user, student, relationship, role, created, updated \
         FROM guardians WHERE school = ? AND user = ? AND student = ?",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(user)
    .bind::<diesel::sql_types::Integer, _>(student)
    .load::<GuardianRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_guardian failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("guardian not found: {school}|{user}|{student}");
        Error::Internal
    })
}

// ---------------------------------------------------------------------------
// Helpers to build ActionRow entries
// ---------------------------------------------------------------------------

fn fetch_student(conn: &mut Conn, school: &str, adm: i32) -> Result<StudentRow> {
    sql_query(
        "SELECT school, adm, user, name, dob, gender, documents, admitted, status, created, updated \
         FROM students WHERE school = ? AND adm = ?",
    )
    .bind::<Text, _>(school)
    .bind::<diesel::sql_types::Integer, _>(adm)
    .load::<StudentRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_student failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("student not found: {school}|{adm}");
        Error::Internal
    })
}

fn fetch_enrollment(
    conn: &mut Conn,
    school: &str,
    year: i32,
    term: i16,
    grade: i16,
    stream: i16,
    student: i32,
) -> Result<EnrollmentRow> {
    sql_query(
        "SELECT school, year, term, grade, stream, student, created \
         FROM enrollments WHERE school = ? AND year = ? AND term = ? AND grade = ? AND stream = ? AND student = ?",
    )
    .bind::<Text, _>(school)
    .bind::<diesel::sql_types::Integer, _>(year)
    .bind::<diesel::sql_types::SmallInt, _>(term)
    .bind::<diesel::sql_types::SmallInt, _>(grade)
    .bind::<diesel::sql_types::SmallInt, _>(stream)
    .bind::<diesel::sql_types::Integer, _>(student)
    .load::<EnrollmentRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_enrollment failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("enrollment not found: {school}|{year}|{term}|{grade}|{stream}|{student}");
        Error::Internal
    })
}

fn fetch_department(conn: &mut Conn, school: &str, name: &str) -> Result<DepartmentRow> {
    sql_query(
        "SELECT school, name, description, created, updated \
         FROM departments WHERE school = ? AND name = ?",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(name)
    .load::<DepartmentRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_department failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("department not found: {school}|{name}");
        Error::Internal
    })
}

fn fetch_term(conn: &mut Conn, school: &str, year: i32, term: i16) -> Result<TermRow> {
    sql_query(
        "SELECT school, year, term, start, \"end\", created, updated \
         FROM terms WHERE school = ? AND year = ? AND term = ?",
    )
    .bind::<Text, _>(school)
    .bind::<diesel::sql_types::Integer, _>(year)
    .bind::<diesel::sql_types::SmallInt, _>(term)
    .load::<TermRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_term failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("term not found: {school}|{year}|{term}");
        Error::Internal
    })
}

// ---------------------------------------------------------------------------
// Additional fetch helpers for L7 tables
// ---------------------------------------------------------------------------

fn fetch_class_teacher(
    conn: &mut Conn,
    school: &str,
    year: i32,
    term: i16,
    grade: i16,
    stream: i16,
    teacher: &str,
) -> Result<ClassTeacherRow> {
    sql_query(
        "SELECT school, year, term, grade, stream, teacher, start, \"end\", created \
         FROM class_teachers WHERE school = ? AND year = ? AND term = ? AND grade = ? AND stream = ? AND teacher = ?",
    )
    .bind::<Text, _>(school)
    .bind::<diesel::sql_types::Integer, _>(year)
    .bind::<diesel::sql_types::SmallInt, _>(term)
    .bind::<diesel::sql_types::SmallInt, _>(grade)
    .bind::<diesel::sql_types::SmallInt, _>(stream)
    .bind::<Text, _>(teacher)
    .load::<ClassTeacherRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_class_teacher failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("class_teacher not found: {school}|{year}|{term}|{grade}|{stream}|{teacher}");
        Error::Internal
    })
}

fn fetch_subject_teacher(
    conn: &mut Conn,
    school: &str,
    year: i32,
    term: i16,
    grade: i16,
    stream: i16,
    subject: i32,
) -> Result<SubjectTeacherRow> {
    sql_query(
        "SELECT school, year, term, grade, stream, subject, teacher, created \
         FROM subject_teachers WHERE school = ? AND year = ? AND term = ? AND grade = ? AND stream = ? AND subject = ?",
    )
    .bind::<Text, _>(school)
    .bind::<diesel::sql_types::Integer, _>(year)
    .bind::<diesel::sql_types::SmallInt, _>(term)
    .bind::<diesel::sql_types::SmallInt, _>(grade)
    .bind::<diesel::sql_types::SmallInt, _>(stream)
    .bind::<diesel::sql_types::Integer, _>(subject)
    .load::<SubjectTeacherRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_subject_teacher failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("subject_teacher not found: {school}|{year}|{term}|{grade}|{stream}|{subject}");
        Error::Internal
    })
}

fn fetch_timetable(
    conn: &mut Conn,
    school: &str,
    year: i32,
    term: i16,
    grade: i16,
    stream: i16,
    subject: i16,
    day: i16,
    start: i32,
) -> Result<TimetableRow> {
    sql_query(
        "SELECT school, year, term, grade, stream, subject, teacher, day, start, \"end\", created, updated \
         FROM timetable WHERE school = ? AND year = ? AND term = ? AND grade = ? AND stream = ? \
         AND subject = ? AND day = ? AND start = ?",
    )
    .bind::<Text, _>(school)
    .bind::<diesel::sql_types::Integer, _>(year)
    .bind::<diesel::sql_types::SmallInt, _>(term)
    .bind::<diesel::sql_types::SmallInt, _>(grade)
    .bind::<diesel::sql_types::SmallInt, _>(stream)
    .bind::<diesel::sql_types::SmallInt, _>(subject)
    .bind::<diesel::sql_types::SmallInt, _>(day)
    .bind::<diesel::sql_types::Integer, _>(start)
    .load::<TimetableRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_timetable failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("timetable not found: {school}|{year}|{term}|{grade}|{stream}|{subject}|{day}|{start}");
        Error::Internal
    })
}

fn fetch_attendance(
    conn: &mut Conn,
    school: &str,
    year: i32,
    term: i16,
    grade: i16,
    stream: i16,
    student: i32,
    date: i32,
) -> Result<AttendanceRow> {
    sql_query(
        "SELECT school, year, term, grade, stream, student, date, status, created, updated \
         FROM attendance WHERE school = ? AND year = ? AND term = ? AND grade = ? AND stream = ? \
         AND student = ? AND date = ?",
    )
    .bind::<Text, _>(school)
    .bind::<diesel::sql_types::Integer, _>(year)
    .bind::<diesel::sql_types::SmallInt, _>(term)
    .bind::<diesel::sql_types::SmallInt, _>(grade)
    .bind::<diesel::sql_types::SmallInt, _>(stream)
    .bind::<diesel::sql_types::Integer, _>(student)
    .bind::<diesel::sql_types::Integer, _>(date)
    .load::<AttendanceRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_attendance failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!(
            "attendance not found: {school}|{year}|{term}|{grade}|{stream}|{student}|{date}"
        );
        Error::Internal
    })
}

fn fetch_lesson(
    conn: &mut Conn,
    school: &str,
    year: i32,
    term: i16,
    grade: i16,
    stream: i16,
    date: i32,
    subject: i16,
    teacher: &str,
) -> Result<LessonRow> {
    sql_query(
        "SELECT school, year, term, grade, stream, date, subject, teacher, created, updated \
         FROM lessons WHERE school = ? AND year = ? AND term = ? AND grade = ? AND stream = ? \
         AND date = ? AND subject = ? AND teacher = ?",
    )
    .bind::<Text, _>(school)
    .bind::<diesel::sql_types::Integer, _>(year)
    .bind::<diesel::sql_types::SmallInt, _>(term)
    .bind::<diesel::sql_types::SmallInt, _>(grade)
    .bind::<diesel::sql_types::SmallInt, _>(stream)
    .bind::<diesel::sql_types::Integer, _>(date)
    .bind::<diesel::sql_types::SmallInt, _>(subject)
    .bind::<Text, _>(teacher)
    .load::<LessonRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_lesson failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!(
            "lesson not found: {school}|{year}|{term}|{grade}|{stream}|{date}|{subject}|{teacher}"
        );
        Error::Internal
    })
}

fn fetch_exam(conn: &mut Conn, id: &str) -> Result<ExamRow> {
    sql_query(
        "SELECT id, school, name, year, term, personalized, \"type\", start, \"end\", teacher, created, updated \
         FROM exams WHERE id = ?",
    )
    .bind::<Text, _>(id)
    .load::<ExamRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_exam failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("exam not found: {id}");
        Error::Internal
    })
}

fn fetch_paper(
    conn: &mut Conn,
    school: &str,
    exam: &str,
    subject: i32,
    paper: Option<i16>,
    grade: i16,
    stream: Option<i16>,
) -> Result<PaperRow> {
    sql_query(
        "SELECT school, exam, subject, paper, topic, invigilator, start, \"end\", status, grade, stream, created, updated \
         FROM papers WHERE school = ? AND exam = ? AND subject = ? AND paper IS ? AND grade = ? AND stream IS ?",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(exam)
    .bind::<diesel::sql_types::Integer, _>(subject)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::SmallInt>, _>(paper)
    .bind::<diesel::sql_types::SmallInt, _>(grade)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::SmallInt>, _>(stream)
    .load::<PaperRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_paper failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("paper not found: {school}|{exam}|{subject}|{paper:?}|{grade}|{stream:?}");
        Error::Internal
    })
}

fn fetch_grade(
    conn: &mut Conn,
    school: &str,
    exam: &str,
    student: i32,
    subject: i32,
    paper: Option<i16>,
) -> Result<GradeRow> {
    sql_query(
        "SELECT school, exam, student, subject, paper, score, total, created, updated \
         FROM grades WHERE school = ? AND exam = ? AND student = ? AND subject = ? AND paper IS ?",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(exam)
    .bind::<diesel::sql_types::Integer, _>(student)
    .bind::<diesel::sql_types::Integer, _>(subject)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::SmallInt>, _>(paper)
    .load::<GradeRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_grade failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("grade not found: {school}|{exam}|{student}|{subject}|{paper:?}");
        Error::Internal
    })
}

fn fetch_mastery(
    conn: &mut Conn,
    school: &str,
    student: i32,
    subject: i32,
    topic: i32,
) -> Result<MasteryRow> {
    sql_query(
        "SELECT school, student, subject, topic, score, created, updated \
         FROM mastery WHERE school = ? AND student = ? AND subject = ? AND topic = ?",
    )
    .bind::<Text, _>(school)
    .bind::<diesel::sql_types::Integer, _>(student)
    .bind::<diesel::sql_types::Integer, _>(subject)
    .bind::<diesel::sql_types::Integer, _>(topic)
    .load::<MasteryRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_mastery failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("mastery not found: {school}|{student}|{subject}|{topic}");
        Error::Internal
    })
}

fn fetch_subject_catalog(conn: &mut Conn, id: i32) -> Result<SubjectCatalogRow> {
    sql_query("SELECT id, name, curriculum, created, updated FROM subjects WHERE id = ?")
        .bind::<diesel::sql_types::Integer, _>(id)
        .load::<SubjectCatalogRow>(conn)
        .map_err(|e| {
            tracing::error!("fetch_subject_catalog failed: {e}");
            Error::Internal
        })?
        .into_iter()
        .next()
        .ok_or_else(|| {
            tracing::error!("subject not found: {id}");
            Error::Internal
        })
}

fn fetch_topic(conn: &mut Conn, id: i32) -> Result<TopicRow> {
    sql_query("SELECT id, subject, grade, name, created, updated FROM topics WHERE id = ?")
        .bind::<diesel::sql_types::Integer, _>(id)
        .load::<TopicRow>(conn)
        .map_err(|e| {
            tracing::error!("fetch_topic failed: {e}");
            Error::Internal
        })?
        .into_iter()
        .next()
        .ok_or_else(|| {
            tracing::error!("topic not found: {id}");
            Error::Internal
        })
}

fn fetch_stream(conn: &mut Conn, school: &str, grade: i16, stream: i16) -> Result<StreamRow> {
    sql_query(
        "SELECT school, grade, stream, name, created, updated FROM streams WHERE school = ? AND grade = ? AND stream = ?",
    )
    .bind::<Text, _>(school)
    .bind::<diesel::sql_types::SmallInt, _>(grade)
    .bind::<diesel::sql_types::SmallInt, _>(stream)
    .load::<StreamRow>(conn)
    .map_err(|e| { tracing::error!("fetch_stream failed: {e}"); Error::Internal })?
    .into_iter()
    .next()
    .ok_or_else(|| { tracing::error!("stream not found: {school}|{grade}|{stream}"); Error::Internal })
}

fn fetch_mpesa(conn: &mut Conn, school: &str) -> Result<MpesaRow> {
    sql_query(
        "SELECT school, consumer_key, consumer_secret, passkey, shortcode, env, created, updated FROM mpesa WHERE school = ?",
    )
    .bind::<Text, _>(school)
    .load::<MpesaRow>(conn)
    .map_err(|e| { tracing::error!("fetch_mpesa failed: {e}"); Error::Internal })?
    .into_iter()
    .next()
    .ok_or_else(|| { tracing::error!("mpesa not found: {school}"); Error::Internal })
}

// ---------------------------------------------------------------------------
// Fetch helpers — Finance, Announcements, Roles, AI, Subscriptions, Discounts
// ---------------------------------------------------------------------------

fn fetch_fee(conn: &mut Conn, id: &str) -> Result<FeeRow> {
    sql_query(
        "SELECT id, school, year, term, grade, title, description, amount, mandatory, due, created, updated \
         FROM fees WHERE id = ?",
    )
    .bind::<Text, _>(id)
    .load::<FeeRow>(conn)
    .map_err(|e| { tracing::error!("fetch_fee failed: {e}"); Error::Internal })?
    .into_iter()
    .next()
    .ok_or_else(|| { tracing::error!("fee not found: {id}"); Error::Internal })
}

fn fetch_invoice(conn: &mut Conn, id: &str) -> Result<InvoiceRow> {
    sql_query(
        "SELECT id, school, year, term, fee, description, student, amount, status, due, created, updated \
         FROM invoices WHERE id = ?",
    )
    .bind::<Text, _>(id)
    .load::<InvoiceRow>(conn)
    .map_err(|e| { tracing::error!("fetch_invoice failed: {e}"); Error::Internal })?
    .into_iter()
    .next()
    .ok_or_else(|| { tracing::error!("invoice not found: {id}"); Error::Internal })
}

fn fetch_payment(conn: &mut Conn, id: &str) -> Result<PaymentRow> {
    sql_query(
        "SELECT id, invoice, school, student, amount, method, reference, recorder, date, created, updated \
         FROM payments WHERE id = ?",
    )
    .bind::<Text, _>(id)
    .load::<PaymentRow>(conn)
    .map_err(|e| { tracing::error!("fetch_payment failed: {e}"); Error::Internal })?
    .into_iter()
    .next()
    .ok_or_else(|| { tracing::error!("payment not found: {id}"); Error::Internal })
}

fn fetch_announcement(conn: &mut Conn, id: &str) -> Result<AnnouncementRow> {
    sql_query(
        "SELECT id, school, title, content, grade, stream, audience, author, created, updated \
         FROM announcements WHERE id = ?",
    )
    .bind::<Text, _>(id)
    .load::<AnnouncementRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_announcement failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("announcement not found: {id}");
        Error::Internal
    })
}

fn fetch_role(conn: &mut Conn, id: &str) -> Result<RoleRow> {
    sql_query(
        "SELECT id, school, name, description, permissions, created, updated \
         FROM roles WHERE id = ?",
    )
    .bind::<Text, _>(id)
    .load::<RoleRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_role failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("role not found: {id}");
        Error::Internal
    })
}

fn fetch_scope(conn: &mut Conn, school: Option<&str>, user: &str, role: &str) -> Result<ScopeRow> {
    let rows = if let Some(s) = school {
        sql_query(
            "SELECT school, user, role, created FROM scopes WHERE school = ? AND user = ? AND role = ?",
        )
        .bind::<Text, _>(s)
        .bind::<Text, _>(user)
        .bind::<Text, _>(role)
        .load::<ScopeRow>(conn)
    } else {
        sql_query(
            "SELECT school, user, role, created FROM scopes WHERE school IS NULL AND user = ? AND role = ?",
        )
        .bind::<Text, _>(user)
        .bind::<Text, _>(role)
        .load::<ScopeRow>(conn)
    };
    rows.map_err(|e| {
        tracing::error!("fetch_scope failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("scope not found: {user}/{role}");
        Error::Internal
    })
}

fn fetch_ai_usage(
    conn: &mut Conn,
    school: &str,
    student: i32,
    year: i32,
    term: i16,
) -> Result<AiUsageRow> {
    sql_query(
        "SELECT school, student, year, term, allocated, used, created, updated \
         FROM aiusage WHERE school = ? AND student = ? AND year = ? AND term = ?",
    )
    .bind::<Text, _>(school)
    .bind::<diesel::sql_types::Integer, _>(student)
    .bind::<diesel::sql_types::Integer, _>(year)
    .bind::<diesel::sql_types::SmallInt, _>(term)
    .load::<AiUsageRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_ai_usage failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("ai_usage not found: {school}/{student}/{year}/{term}");
        Error::Internal
    })
}

fn fetch_subscription(
    conn: &mut Conn,
    school: &str,
    plan: &str,
    year: i32,
    term: i16,
    student: i32,
) -> Result<SubscriptionRow> {
    sql_query(
        "SELECT school, plan, year, term, student, invoice, discount, status, created, updated \
         FROM subscriptions WHERE school = ? AND plan = ? AND year = ? AND term = ? AND student = ?",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(plan)
    .bind::<diesel::sql_types::Integer, _>(year)
    .bind::<diesel::sql_types::SmallInt, _>(term)
    .bind::<diesel::sql_types::Integer, _>(student)
    .load::<SubscriptionRow>(conn)
    .map_err(|e| { tracing::error!("fetch_subscription failed: {e}"); Error::Internal })?
    .into_iter()
    .next()
    .ok_or_else(|| { tracing::error!("subscription not found"); Error::Internal })
}

fn fetch_discount(
    conn: &mut Conn,
    school: &str,
    plan: &str,
    year: i32,
    term: i16,
    grade: i16,
) -> Result<DiscountRow> {
    sql_query(
        "SELECT school, plan, year, term, grade, amount, unit, created, updated \
         FROM discounts WHERE school = ? AND plan = ? AND year = ? AND term = ? AND grade = ?",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(plan)
    .bind::<diesel::sql_types::Integer, _>(year)
    .bind::<diesel::sql_types::SmallInt, _>(term)
    .bind::<diesel::sql_types::SmallInt, _>(grade)
    .load::<DiscountRow>(conn)
    .map_err(|e| {
        tracing::error!("fetch_discount failed: {e}");
        Error::Internal
    })?
    .into_iter()
    .next()
    .ok_or_else(|| {
        tracing::error!("discount not found");
        Error::Internal
    })
}

fn upsert_row(table: i32, row_key: String, data: InsertData) -> ActionRow {
    ActionRow {
        table,
        operation: 0, // upsert
        row_key,
        data: Some(data),
    }
}

fn delete_row(table: i32, row_key: String) -> ActionRow {
    ActionRow {
        table,
        operation: 2, // delete
        row_key,
        data: None,
    }
}

fn append_log(user: Id, table: u8, op: u8, columns: u16) -> Result<()> {
    let record = Record::new(user, table, op, columns);
    LOG.with(|cell| cell.borrow_mut().append(&record))
        .map_err(|e| {
            tracing::error!("changelog append failed: {e}");
            Error::Internal
        })?;
    Ok(())
}

fn append_delete_log(table: u8, row_key: &str) -> Result<()> {
    LOG.with(|cell| cell.borrow_mut().append_delete(table, row_key))
        .map_err(|e| {
            tracing::error!("changelog append_delete failed: {e}");
            Error::Internal
        })?;
    Ok(())
}

/// Central dispatcher: deserialize payload, execute, return rows.
///
/// Authorization is handled by the caller (the push flow in
/// `services/sync.rs`). By the time we get here, permission has already
/// been checked.
pub fn execute_action(conn: &mut Conn, action_id: i32, payload: &[u8]) -> Result<ActionResult> {
    use sync_action::*;
    match action_id {
        // Schools
        CREATE_SCHOOL => handle_create_school(conn, payload),
        UPDATE_SCHOOL => handle_update_school(conn, payload),
        DELETE_SCHOOL => handle_delete_school(conn, payload),

        // Teachers
        CREATE_TEACHER => handle_create_teacher(conn, payload),
        UPDATE_TEACHER => handle_update_teacher(conn, payload),
        DELETE_TEACHER => handle_delete_teacher(conn, payload),

        // Staff
        CREATE_STAFF => handle_create_staff(conn, payload),
        UPDATE_STAFF => handle_update_staff(conn, payload),
        DELETE_STAFF => handle_delete_staff(conn, payload),

        // Owners
        CREATE_OWNER => handle_create_owner(conn, payload),
        DELETE_OWNER => handle_delete_owner(conn, payload),

        // Students
        CREATE_STUDENT => handle_create_student(conn, payload),
        UPDATE_STUDENT => handle_update_student(conn, payload),
        DELETE_STUDENT => handle_delete_student(conn, payload),
        ENROLL_STUDENT => handle_enroll_student(conn, payload),
        UNENROLL_STUDENT => handle_unenroll_student(conn, payload),

        // Guardians
        CREATE_GUARDIAN => handle_create_guardian(conn, payload),
        UPDATE_GUARDIAN => handle_update_guardian(conn, payload),
        DELETE_GUARDIAN => handle_delete_guardian(conn, payload),

        // Departments
        CREATE_DEPARTMENT => handle_create_department(conn, payload),
        UPDATE_DEPARTMENT => handle_update_department(conn, payload),
        DELETE_DEPARTMENT => handle_delete_department(conn, payload),

        // Terms
        CREATE_TERM => handle_create_term(conn, payload),
        UPDATE_TERM => handle_update_term(conn, payload),
        DELETE_TERM => handle_delete_term(conn, payload),

        // Class teachers
        ASSIGN_CLASS_TEACHER => handle_assign_class_teacher(conn, payload),
        UNASSIGN_CLASS_TEACHER => handle_unassign_class_teacher(conn, payload),

        // Subjects
        ASSIGN_SUBJECT => handle_assign_subject(conn, payload),
        UNASSIGN_SUBJECT => handle_unassign_subject(conn, payload),

        // Timetable
        CREATE_TIMETABLE_ENTRY => handle_create_timetable_entry(conn, payload),
        UPDATE_TIMETABLE_ENTRY => handle_update_timetable_entry(conn, payload),
        DELETE_TIMETABLE_ENTRY => handle_delete_timetable_entry(conn, payload),

        // Attendance
        MARK_ATTENDANCE => handle_mark_attendance(conn, payload),
        DELETE_ATTENDANCE => handle_delete_attendance(conn, payload),

        // Lessons
        CREATE_LESSON => handle_create_lesson(conn, payload),
        DELETE_LESSON => handle_delete_lesson(conn, payload),

        // Exams
        CREATE_EXAM => handle_create_exam(conn, payload),
        UPDATE_EXAM => handle_update_exam(conn, payload),
        DELETE_EXAM => handle_delete_exam(conn, payload),

        // Papers
        CREATE_PAPER => handle_create_paper(conn, payload),
        UPDATE_PAPER => handle_update_paper(conn, payload),
        DELETE_PAPER => handle_delete_paper(conn, payload),

        // Grades
        MARK_GRADES => handle_mark_grades(conn, payload),
        UPDATE_GRADE => handle_update_grade(conn, payload),
        DELETE_GRADE => handle_delete_grade(conn, payload),

        // Mastery
        UPDATE_MASTERY => handle_update_mastery(conn, payload),

        // Fees
        CREATE_FEE => handle_create_fee(conn, payload),
        UPDATE_FEE => handle_update_fee(conn, payload),
        DELETE_FEE => handle_delete_fee(conn, payload),

        // Invoices
        CREATE_INVOICE => handle_create_invoice(conn, payload),
        UPDATE_INVOICE => handle_update_invoice(conn, payload),
        DELETE_INVOICE => handle_delete_invoice(conn, payload),

        // Payments
        CREATE_PAYMENT => handle_create_payment(conn, payload),
        UPDATE_PAYMENT => handle_update_payment(conn, payload),
        DELETE_PAYMENT => handle_delete_payment(conn, payload),
        APPROVE_PAYMENT => handle_approve_payment(conn, payload),

        // Announcements
        CREATE_ANNOUNCEMENT => handle_create_announcement(conn, payload),
        UPDATE_ANNOUNCEMENT => handle_update_announcement(conn, payload),
        DELETE_ANNOUNCEMENT => handle_delete_announcement(conn, payload),

        // Roles
        CREATE_ROLE => handle_create_role(conn, payload),
        UPDATE_ROLE => handle_update_role(conn, payload),
        DELETE_ROLE => handle_delete_role(conn, payload),
        ASSIGN_ROLE => handle_assign_role(conn, payload),
        UNASSIGN_ROLE => handle_unassign_role(conn, payload),

        // Users
        UPDATE_USER => handle_update_user(conn, payload),
        DELETE_USER => handle_delete_user(conn, payload),

        // Plans
        CREATE_PLAN => handle_create_plan(conn, payload),
        UPDATE_PLAN => handle_update_plan(conn, payload),
        DELETE_PLAN => handle_delete_plan(conn, payload),

        // AI usage
        UPDATE_AI_USAGE => handle_update_ai_usage(conn, payload),

        // Subscriptions
        CREATE_SUBSCRIPTION => handle_create_subscription(conn, payload),
        UPDATE_SUBSCRIPTION => handle_update_subscription(conn, payload),
        DELETE_SUBSCRIPTION => handle_delete_subscription(conn, payload),

        // Discounts
        CREATE_DISCOUNT => handle_create_discount(conn, payload),
        UPDATE_DISCOUNT => handle_update_discount(conn, payload),
        DELETE_DISCOUNT => handle_delete_discount(conn, payload),

        // Subjects (global catalog)
        CREATE_SUBJECT => handle_create_subject(conn, payload),
        UPDATE_SUBJECT => handle_update_subject(conn, payload),
        DELETE_SUBJECT => handle_delete_subject(conn, payload),

        // Topics (global catalog)
        CREATE_TOPIC => handle_create_topic(conn, payload),
        UPDATE_TOPIC => handle_update_topic(conn, payload),
        DELETE_TOPIC => handle_delete_topic(conn, payload),

        // Streams
        CREATE_STREAM => handle_create_stream(conn, payload),
        UPDATE_STREAM => handle_update_stream(conn, payload),
        DELETE_STREAM => handle_delete_stream(conn, payload),

        // Mpesa
        CREATE_MPESA => handle_create_mpesa(conn, payload),
        UPDATE_MPESA => handle_update_mpesa(conn, payload),
        DELETE_MPESA => handle_delete_mpesa(conn, payload),

        // File sync: scheme & answer pages
        UPLOAD_SCHEME => handle_upload_scheme(conn, payload),
        DELETE_SCHEME => handle_delete_scheme(conn, payload),
        UPLOAD_ANSWER_SHEET => handle_upload_answer_sheet(conn, payload),
        DELETE_ANSWER_SHEET => handle_delete_answer_sheet(conn, payload),

        // 89, 90: reserved (removed exam_grade actions)
        _ => {
            tracing::error!("execute_action: unknown action {action_id}");
            Err(Error::Internal)
        }
    }
}

// ---------------------------------------------------------------------------
// Schools
// ---------------------------------------------------------------------------

fn handle_create_school(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateSchoolPayload = decode(payload)?;

    // Use a dummy user ID for changelog — the caller (push flow) will
    // overwrite with the real authenticated user once authorization is wired.
    let log_user = Id::system();

    // 1. Look up owner by phone — if not found, create an invited user.
    let owner_user = match fetch_user_by_phone(conn, &p.owner_phone)? {
        Some(existing) => existing,
        None => {
            let user_insert = UserInsert {
                id: p.owner_id.clone(),
                phone: p.owner_phone.clone(),
                email: p.owner_email.clone(),
                name: p.owner_name.clone(),
                level: 0,  // Normal
                status: 0, // Invited
            };
            insert::insert_user(conn, &user_insert)?;
            append_log(log_user, TBL_USERS as u8, OP_INSERT, 0)?;
            fetch_user(conn, &p.owner_id)?
        }
    };

    // 2. Insert school.
    let school_insert = SchoolInsert {
        id: p.id.clone(),
        name: p.name.clone(),
        motto: p.motto.clone(),
        phone: p.phone.clone(),
        email: p.email.clone(),
        county: p.county,
        domain: p.domain.clone(),
        established: p.established,
        status: 0, // Active
    };
    insert::insert_school(conn, &school_insert)?;
    append_log(log_user, TBL_SCHOOLS as u8, OP_INSERT, 0)?;

    // 3. Insert owner record.
    let owner_insert = OwnerInsert {
        school: p.id.clone(),
        user: owner_user.id.clone(),
    };
    insert::insert_owner(conn, &owner_insert)?;
    append_log(log_user, TBL_OWNERS as u8, OP_INSERT, 0)?;

    // 4. Fetch all created rows and build response.
    let user_row = fetch_user(conn, &owner_user.id)?;
    let school_row = fetch_school(conn, &p.id)?;
    let owner_row = fetch_owner(conn, &p.id, &owner_user.id)?;

    let rows = vec![
        upsert_row(
            TBL_USERS,
            user_row.row_key(),
            InsertData {
                row: Some(insert_data::Row::User((&user_row).into())),
            },
        ),
        upsert_row(
            TBL_SCHOOLS,
            school_row.row_key(),
            InsertData {
                row: Some(insert_data::Row::School((&school_row).into())),
            },
        ),
        upsert_row(
            TBL_OWNERS,
            owner_row.row_key(),
            InsertData {
                row: Some(insert_data::Row::Owner((&owner_row).into())),
            },
        ),
    ];

    Ok(ActionResult::with_rows(rows))
}

fn handle_update_school(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateSchoolPayload = decode(payload)?;
    let log_user = Id::system();

    update::update_school(conn, &p.id, &p)?;
    append_log(log_user, TBL_SCHOOLS as u8, OP_UPDATE, 0)?;

    let row = fetch_school(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_SCHOOLS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::School((&row).into())),
        },
    )]))
}

fn handle_delete_school(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteSchoolPayload = decode(payload)?;
    let log_user = Id::system();

    // Soft-delete: set status to deleted (status = 2).
    let soft_delete = UpdateSchoolPayload {
        id: p.id.clone(),
        name: None,
        motto: None,
        phone: None,
        email: None,
        county: None,
        domain: None,
        established: None,
        status: Some(2), // Deleted
    };
    update::update_school(conn, &p.id, &soft_delete)?;
    append_log(log_user, TBL_SCHOOLS as u8, OP_UPDATE, 0)?;

    let row = fetch_school(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_SCHOOLS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::School((&row).into())),
        },
    )]))
}

// ---------------------------------------------------------------------------
// Teachers
// ---------------------------------------------------------------------------

fn handle_create_teacher(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateTeacherPayload = decode(payload)?;
    let log_user = Id::system();

    // Invitation pattern: look up user by phone, create if not found.
    let user_row = match fetch_user_by_phone(conn, &p.phone)? {
        Some(existing) => existing,
        None => {
            let user_insert = UserInsert {
                id: p.user_id.clone(),
                phone: p.phone.clone(),
                email: p.email.clone(),
                name: p.name.clone(),
                level: 0,  // Normal
                status: 0, // Invited
            };
            insert::insert_user(conn, &user_insert)?;
            append_log(log_user, TBL_USERS as u8, OP_INSERT, 0)?;
            fetch_user(conn, &p.user_id)?
        }
    };

    // Insert teacher record pointing to the resolved user.
    let teacher_insert = TeacherInsert {
        school: p.school.clone(),
        user: user_row.id.clone(),
        hired: p.hired,
        role: p.role.clone(),
        department: p.department.clone(),
        status: 0, // Active
    };
    insert::insert_teacher(conn, &teacher_insert)?;
    append_log(log_user, TBL_TEACHERS as u8, OP_INSERT, 0)?;

    // Fetch and return both rows.
    let user_row = fetch_user(conn, &user_row.id)?;
    let teacher_row = fetch_teacher(conn, &p.school, &user_row.id)?;

    Ok(ActionResult::with_rows(vec![
        upsert_row(
            TBL_USERS,
            user_row.row_key(),
            InsertData {
                row: Some(insert_data::Row::User((&user_row).into())),
            },
        ),
        upsert_row(
            TBL_TEACHERS,
            teacher_row.row_key(),
            InsertData {
                row: Some(insert_data::Row::Teacher((&teacher_row).into())),
            },
        ),
    ]))
}

fn handle_update_teacher(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateTeacherPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}", p.school, p.user);

    update::update_teacher(conn, &row_key, &p)?;
    append_log(log_user, TBL_TEACHERS as u8, OP_UPDATE, 0)?;

    let row = fetch_teacher(conn, &p.school, &p.user)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_TEACHERS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Teacher((&row).into())),
        },
    )]))
}

fn handle_delete_teacher(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteTeacherPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}", p.school, p.user);

    delete::delete_teacher(conn, &row_key)?;
    append_log(log_user, TBL_TEACHERS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_TEACHERS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_TEACHERS,
        row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Staff
// ---------------------------------------------------------------------------

fn handle_create_staff(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateStaffPayload = decode(payload)?;
    let log_user = Id::system();

    // Invitation pattern: look up user by phone, create if not found.
    let user_row = match fetch_user_by_phone(conn, &p.phone)? {
        Some(existing) => existing,
        None => {
            let user_insert = UserInsert {
                id: p.user_id.clone(),
                phone: p.phone.clone(),
                email: p.email.clone(),
                name: p.name.clone(),
                level: 0,  // Normal
                status: 0, // Invited
            };
            insert::insert_user(conn, &user_insert)?;
            append_log(log_user, TBL_USERS as u8, OP_INSERT, 0)?;
            fetch_user(conn, &p.user_id)?
        }
    };

    // Insert staff record pointing to the resolved user.
    let staff_insert = StaffInsert {
        school: p.school.clone(),
        user: user_row.id.clone(),
        idnumber: p.idnumber.clone(),
        role: p.role.clone(),
        department: p.department.clone(),
        status: 0, // Active
    };
    insert::insert_staff(conn, &staff_insert)?;
    append_log(log_user, TBL_STAFF as u8, OP_INSERT, 0)?;

    // Fetch and return both rows.
    let user_row = fetch_user(conn, &user_row.id)?;
    let staff_row = fetch_staff(conn, &p.school, &user_row.id)?;

    Ok(ActionResult::with_rows(vec![
        upsert_row(
            TBL_USERS,
            user_row.row_key(),
            InsertData {
                row: Some(insert_data::Row::User((&user_row).into())),
            },
        ),
        upsert_row(
            TBL_STAFF,
            staff_row.row_key(),
            InsertData {
                row: Some(insert_data::Row::StaffMember((&staff_row).into())),
            },
        ),
    ]))
}

fn handle_update_staff(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateStaffPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}", p.school, p.user);

    update::update_staff(conn, &row_key, &p)?;
    append_log(log_user, TBL_STAFF as u8, OP_UPDATE, 0)?;

    let row = fetch_staff(conn, &p.school, &p.user)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_STAFF,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::StaffMember((&row).into())),
        },
    )]))
}

fn handle_delete_staff(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteStaffPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}", p.school, p.user);

    delete::delete_staff(conn, &row_key)?;
    append_log(log_user, TBL_STAFF as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_STAFF as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_STAFF, row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Owners
// ---------------------------------------------------------------------------

fn handle_create_owner(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateOwnerPayload = decode(payload)?;
    let log_user = Id::system();

    // Invitation pattern: look up user by phone, create if not found.
    let user_row = match fetch_user_by_phone(conn, &p.phone)? {
        Some(existing) => existing,
        None => {
            let user_insert = UserInsert {
                id: p.user_id.clone(),
                phone: p.phone.clone(),
                email: p.email.clone(),
                name: p.name.clone(),
                level: 0,  // Normal
                status: 0, // Invited
            };
            insert::insert_user(conn, &user_insert)?;
            append_log(log_user, TBL_USERS as u8, OP_INSERT, 0)?;
            fetch_user(conn, &p.user_id)?
        }
    };

    // Insert owner record pointing to the resolved user.
    let owner_insert = OwnerInsert {
        school: p.school.clone(),
        user: user_row.id.clone(),
    };
    insert::insert_owner(conn, &owner_insert)?;
    append_log(log_user, TBL_OWNERS as u8, OP_INSERT, 0)?;

    // Fetch and return both rows.
    let user_row = fetch_user(conn, &user_row.id)?;
    let owner_row = fetch_owner(conn, &p.school, &user_row.id)?;

    Ok(ActionResult::with_rows(vec![
        upsert_row(
            TBL_USERS,
            user_row.row_key(),
            InsertData {
                row: Some(insert_data::Row::User((&user_row).into())),
            },
        ),
        upsert_row(
            TBL_OWNERS,
            owner_row.row_key(),
            InsertData {
                row: Some(insert_data::Row::Owner((&owner_row).into())),
            },
        ),
    ]))
}

fn handle_delete_owner(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteOwnerPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}", p.school, p.user);

    delete::delete_owner(conn, &row_key)?;
    append_log(log_user, TBL_OWNERS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_OWNERS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_OWNERS, row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Students
// ---------------------------------------------------------------------------

fn handle_create_student(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateStudentPayload = decode(payload)?;
    let log_user = Id::system();

    let student_insert = StudentInsert {
        school: p.school.clone(),
        adm: p.adm,
        user: p.user.clone(),
        name: p.name.clone(),
        dob: p.dob,
        gender: p.gender,
        documents: p.documents.clone(),
        admitted: p.admitted,
        status: 0, // Active
    };
    insert::insert_student(conn, &student_insert)?;
    append_log(log_user, TBL_STUDENTS as u8, OP_INSERT, 0)?;

    let row = fetch_student(conn, &p.school, p.adm)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_STUDENTS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Student((&row).into())),
        },
    )]))
}

fn handle_update_student(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    use crate::config::storage::sign;
    use chrono::Utc;

    let p: UpdateStudentPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}", p.school, p.adm);

    update::update_student(conn, &row_key, &p)?;
    append_log(log_user, TBL_STUDENTS as u8, OP_UPDATE, 0)?;

    let row = fetch_student(conn, &p.school, p.adm)?;

    // Build the S3 path for this student's profile image.
    // Convention: schools/{school_id}/students/{adm}/image  (no extension)
    let path = format!("schools/{}/students/{}/image", p.school, p.adm);

    // PUT URL — valid 1 hour — for the originator to upload their local image.
    let put_url = sign::url(&path, sign::PUT_TTL, true);
    // GET URL — valid 1 month — for any client to download the image.
    let get_url = sign::url(&path, sign::GET_TTL, false);

    // expiry is milliseconds since epoch when the GET URL expires.
    let expiry_ms = (Utc::now().timestamp() + sign::GET_TTL as i64) * 1000;

    let file_url = FileUrl {
        path,
        put_url: Some(put_url),
        get_url: Some(get_url),
        expiry: expiry_ms,
    };

    Ok(ActionResult::with_rows_and_urls(
        vec![upsert_row(
            TBL_STUDENTS,
            row.row_key(),
            InsertData {
                row: Some(insert_data::Row::Student((&row).into())),
            },
        )],
        vec![file_url],
    ))
}

fn handle_delete_student(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteStudentPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}", p.school, p.adm);

    delete::delete_student(conn, &row_key)?;
    append_log(log_user, TBL_STUDENTS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_STUDENTS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_STUDENTS,
        row_key,
    )]))
}

fn handle_enroll_student(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: EnrollStudentPayload = decode(payload)?;
    let log_user = Id::system();

    let enrollment_insert = EnrollmentInsert {
        school: p.school.clone(),
        year: p.year,
        term: p.term,
        grade: p.grade,
        stream: p.stream,
        student: p.student,
    };
    insert::insert_enrollment(conn, &enrollment_insert)?;
    append_log(log_user, TBL_ENROLLMENTS as u8, OP_INSERT, 0)?;

    let row = fetch_enrollment(
        conn,
        &p.school,
        p.year,
        p.term as i16,
        p.grade as i16,
        p.stream as i16,
        p.student,
    )?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_ENROLLMENTS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Enrollment((&row).into())),
        },
    )]))
}

fn handle_unenroll_student(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UnenrollStudentPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!(
        "{}|{}|{}|{}|{}|{}",
        p.school, p.year, p.term, p.grade, p.stream, p.student
    );

    delete::delete_enrollment(conn, &row_key)?;
    append_log(log_user, TBL_ENROLLMENTS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_ENROLLMENTS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_ENROLLMENTS,
        row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Guardians
// ---------------------------------------------------------------------------

fn handle_create_guardian(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateGuardianPayload = decode(payload)?;
    let log_user = Id::system();

    // Invitation pattern: look up user by phone, create if not found.
    let user_row = match fetch_user_by_phone(conn, &p.phone)? {
        Some(existing) => existing,
        None => {
            let user_insert = UserInsert {
                id: p.user_id.clone(),
                phone: p.phone.clone(),
                email: p.email.clone(),
                name: p.name.clone(),
                level: 0,  // Normal
                status: 0, // Invited
            };
            insert::insert_user(conn, &user_insert)?;
            append_log(log_user, TBL_USERS as u8, OP_INSERT, 0)?;
            fetch_user(conn, &p.user_id)?
        }
    };

    // Insert guardian record pointing to the resolved user.
    let guardian_insert = GuardianInsert {
        school: p.school.clone(),
        user: user_row.id.clone(),
        student: p.student,
        relationship: p.relationship,
        role: p.role,
    };
    insert::insert_guardian(conn, &guardian_insert)?;
    append_log(log_user, TBL_GUARDIANS as u8, OP_INSERT, 0)?;

    // Fetch and return both rows.
    let user_row = fetch_user(conn, &user_row.id)?;
    let guardian_row = fetch_guardian(conn, &p.school, &user_row.id, p.student)?;

    Ok(ActionResult::with_rows(vec![
        upsert_row(
            TBL_USERS,
            user_row.row_key(),
            InsertData {
                row: Some(insert_data::Row::User((&user_row).into())),
            },
        ),
        upsert_row(
            TBL_GUARDIANS,
            guardian_row.row_key(),
            InsertData {
                row: Some(insert_data::Row::Guardian((&guardian_row).into())),
            },
        ),
    ]))
}

fn handle_update_guardian(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateGuardianPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}|{}", p.school, p.user, p.student);

    update::update_guardian(conn, &row_key, &p)?;
    append_log(log_user, TBL_GUARDIANS as u8, OP_UPDATE, 0)?;

    let row = fetch_guardian(conn, &p.school, &p.user, p.student)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_GUARDIANS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Guardian((&row).into())),
        },
    )]))
}

fn handle_delete_guardian(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteGuardianPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}|{}", p.school, p.user, p.student);

    delete::delete_guardian(conn, &row_key)?;
    append_log(log_user, TBL_GUARDIANS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_GUARDIANS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_GUARDIANS,
        row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Departments
// ---------------------------------------------------------------------------

fn handle_create_department(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateDepartmentPayload = decode(payload)?;
    let log_user = Id::system();

    let dept_insert = DepartmentInsert {
        school: p.school.clone(),
        name: p.name.clone(),
        description: p.description.clone(),
    };
    insert::insert_department(conn, &dept_insert)?;
    append_log(log_user, TBL_DEPARTMENTS as u8, OP_INSERT, 0)?;

    let row = fetch_department(conn, &p.school, &p.name)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_DEPARTMENTS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Department((&row).into())),
        },
    )]))
}

fn handle_update_department(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateDepartmentPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}", p.school, p.name);

    update::update_department(conn, &row_key, &p)?;
    append_log(log_user, TBL_DEPARTMENTS as u8, OP_UPDATE, 0)?;

    let row = fetch_department(conn, &p.school, &p.name)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_DEPARTMENTS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Department((&row).into())),
        },
    )]))
}

fn handle_delete_department(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteDepartmentPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}", p.school, p.name);

    delete::delete_department(conn, &row_key)?;
    append_log(log_user, TBL_DEPARTMENTS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_DEPARTMENTS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_DEPARTMENTS,
        row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Terms
// ---------------------------------------------------------------------------

fn handle_create_term(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateTermPayload = decode(payload)?;
    let log_user = Id::system();

    let term_insert = TermInsert {
        school: p.school.clone(),
        year: p.year,
        term: p.term,
        start: p.start,
        end: p.end,
    };
    insert::insert_term(conn, &term_insert)?;
    append_log(log_user, TBL_TERMS as u8, OP_INSERT, 0)?;

    let row = fetch_term(conn, &p.school, p.year, p.term as i16)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_TERMS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Term((&row).into())),
        },
    )]))
}

fn handle_update_term(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateTermPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}|{}", p.school, p.year, p.term);

    update::update_term(conn, &row_key, &p)?;
    append_log(log_user, TBL_TERMS as u8, OP_UPDATE, 0)?;

    let row = fetch_term(conn, &p.school, p.year, p.term as i16)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_TERMS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Term((&row).into())),
        },
    )]))
}

fn handle_delete_term(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteTermPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}|{}", p.school, p.year, p.term);

    delete::delete_term(conn, &row_key)?;
    append_log(log_user, TBL_TERMS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_TERMS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_TERMS, row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Class teachers
// ---------------------------------------------------------------------------

fn handle_assign_class_teacher(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: AssignClassTeacherPayload = decode(payload)?;
    let log_user = Id::system();

    let ct_insert = ClassTeacherInsert {
        school: p.school.clone(),
        year: p.year,
        term: p.term,
        grade: p.grade,
        stream: p.stream,
        teacher: p.teacher.clone(),
        start: p.start,
        end: p.end,
    };
    insert::insert_class_teacher(conn, &ct_insert)?;
    append_log(log_user, TBL_CLASS_TEACHERS as u8, OP_INSERT, 0)?;

    let row = fetch_class_teacher(
        conn,
        &p.school,
        p.year,
        p.term as i16,
        p.grade as i16,
        p.stream as i16,
        &p.teacher,
    )?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_CLASS_TEACHERS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::ClassTeacher((&row).into())),
        },
    )]))
}

fn handle_unassign_class_teacher(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UnassignClassTeacherPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!(
        "{}|{}|{}|{}|{}|{}",
        p.school, p.year, p.term, p.grade, p.stream, p.teacher
    );

    delete::delete_class_teacher(conn, &row_key)?;
    append_log(log_user, TBL_CLASS_TEACHERS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_CLASS_TEACHERS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_CLASS_TEACHERS,
        row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Subjects
// ---------------------------------------------------------------------------

fn handle_assign_subject(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: AssignSubjectPayload = decode(payload)?;
    let log_user = Id::system();

    tracing::info!(
        school = %p.school,
        year = p.year,
        term = p.term,
        grade = p.grade,
        stream = p.stream,
        subject = p.subject,
        teacher = %p.teacher,
        "[ASSIGN_SUBJECT] payload values"
    );

    // Pre-check each FK to pinpoint which one fails
    let school_exists: i64 = diesel::sql_query("SELECT COUNT(*) AS cnt FROM schools WHERE id = ?")
        .bind::<diesel::sql_types::Text, _>(&p.school)
        .load::<FkCheckRow>(conn)
        .map(|r| r.first().map(|row| row.cnt).unwrap_or(0))
        .unwrap_or(0);
    let term_exists: i64 = diesel::sql_query(
        "SELECT COUNT(*) AS cnt FROM terms WHERE school = ? AND year = ? AND term = ?",
    )
    .bind::<diesel::sql_types::Text, _>(&p.school)
    .bind::<diesel::sql_types::Integer, _>(p.year)
    .bind::<diesel::sql_types::SmallInt, _>(p.term as i16)
    .load::<FkCheckRow>(conn)
    .map(|r| r.first().map(|row| row.cnt).unwrap_or(0))
    .unwrap_or(0);
    let teacher_exists: i64 =
        diesel::sql_query("SELECT COUNT(*) AS cnt FROM teachers WHERE school = ? AND user = ?")
            .bind::<diesel::sql_types::Text, _>(&p.school)
            .bind::<diesel::sql_types::Text, _>(&p.teacher)
            .load::<FkCheckRow>(conn)
            .map(|r| r.first().map(|row| row.cnt).unwrap_or(0))
            .unwrap_or(0);
    let subject_exists: i64 =
        diesel::sql_query("SELECT COUNT(*) AS cnt FROM subjects WHERE id = ?")
            .bind::<diesel::sql_types::Integer, _>(p.subject)
            .load::<FkCheckRow>(conn)
            .map(|r| r.first().map(|row| row.cnt).unwrap_or(0))
            .unwrap_or(0);

    tracing::info!(
        school_exists,
        term_exists,
        teacher_exists,
        subject_exists,
        "[ASSIGN_SUBJECT] FK pre-check results"
    );

    let sub_insert = SubjectTeacherInsert {
        school: p.school.clone(),
        year: p.year,
        term: p.term,
        grade: p.grade,
        stream: p.stream,
        subject: p.subject,
        teacher: p.teacher.clone(),
    };
    insert::insert_subject_teacher(conn, &sub_insert)?;
    append_log(log_user, TBL_SUBJECTS as u8, OP_INSERT, 0)?;

    let row = fetch_subject_teacher(
        conn,
        &p.school,
        p.year,
        p.term as i16,
        p.grade as i16,
        p.stream as i16,
        p.subject as i32,
    )?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_SUBJECTS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::SubjectTeacher((&row).into())),
        },
    )]))
}

fn handle_unassign_subject(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UnassignSubjectPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!(
        "{}|{}|{}|{}|{}|{}",
        p.school, p.year, p.term, p.grade, p.stream, p.subject
    );

    delete::delete_subject_teacher(conn, &row_key)?;
    append_log(log_user, TBL_SUBJECTS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_SUBJECTS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_SUBJECTS,
        row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Timetable
// ---------------------------------------------------------------------------

fn handle_create_timetable_entry(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateTimetableEntryPayload = decode(payload)?;
    let log_user = Id::system();

    let tt_insert = TimetableInsert {
        school: p.school.clone(),
        year: p.year,
        term: p.term,
        grade: p.grade,
        stream: p.stream,
        subject: p.subject,
        teacher: p.teacher.clone(),
        day: p.day,
        start: p.start,
        end: p.end,
    };
    insert::insert_timetable(conn, &tt_insert)?;
    append_log(log_user, TBL_TIMETABLE as u8, OP_INSERT, 0)?;

    let row = fetch_timetable(
        conn,
        &p.school,
        p.year,
        p.term as i16,
        p.grade as i16,
        p.stream as i16,
        p.subject as i16,
        p.day as i16,
        p.start,
    )?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_TIMETABLE,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Timetable((&row).into())),
        },
    )]))
}

fn handle_update_timetable_entry(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateTimetableEntryPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        p.school, p.year, p.term, p.grade, p.stream, p.subject, p.day, p.start
    );

    update::update_timetable(conn, &row_key, &p)?;
    append_log(log_user, TBL_TIMETABLE as u8, OP_UPDATE, 0)?;

    let row = fetch_timetable(
        conn,
        &p.school,
        p.year,
        p.term as i16,
        p.grade as i16,
        p.stream as i16,
        p.subject as i16,
        p.day as i16,
        p.start,
    )?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_TIMETABLE,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Timetable((&row).into())),
        },
    )]))
}

fn handle_delete_timetable_entry(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteTimetableEntryPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        p.school, p.year, p.term, p.grade, p.stream, p.subject, p.day, p.start
    );

    delete::delete_timetable(conn, &row_key)?;
    append_log(log_user, TBL_TIMETABLE as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_TIMETABLE as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_TIMETABLE,
        row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Attendance
// ---------------------------------------------------------------------------

fn handle_mark_attendance(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: MarkAttendancePayload = decode(payload)?;
    let log_user = Id::system();
    let mut rows = Vec::new();

    for rec in &p.records {
        let att_insert = AttendanceInsert {
            school: p.school.clone(),
            year: p.year,
            term: p.term,
            grade: p.grade,
            stream: p.stream,
            student: rec.student,
            date: p.date,
            status: rec.status,
        };
        // Upsert: try insert, on conflict update status
        let row_key = format!(
            "{}|{}|{}|{}|{}|{}|{}",
            p.school, p.year, p.term, p.grade, p.stream, rec.student, p.date
        );
        let inserted = insert::insert_attendance(conn, &att_insert);
        if inserted.is_err() {
            // Row already exists — update instead
            update::update_attendance(conn, &row_key, rec.status as i16)?;
            append_log(log_user, TBL_ATTENDANCE as u8, OP_UPDATE, 0)?;
        } else {
            append_log(log_user, TBL_ATTENDANCE as u8, OP_INSERT, 0)?;
        }

        let row = fetch_attendance(
            conn,
            &p.school,
            p.year,
            p.term as i16,
            p.grade as i16,
            p.stream as i16,
            rec.student,
            p.date,
        )?;
        rows.push(upsert_row(
            TBL_ATTENDANCE,
            row.row_key(),
            InsertData {
                row: Some(insert_data::Row::Attendance((&row).into())),
            },
        ));
    }

    Ok(ActionResult::with_rows(rows))
}

fn handle_delete_attendance(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteAttendancePayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        p.school, p.year, p.term, p.grade, p.stream, p.student, p.date
    );

    delete::delete_attendance(conn, &row_key)?;
    append_log(log_user, TBL_ATTENDANCE as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_ATTENDANCE as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_ATTENDANCE,
        row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Lessons
// ---------------------------------------------------------------------------

fn handle_create_lesson(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateLessonPayload = decode(payload)?;
    let log_user = Id::system();

    let lesson_insert = LessonInsert {
        school: p.school.clone(),
        year: p.year,
        term: p.term,
        grade: p.grade,
        stream: p.stream,
        date: p.date,
        subject: p.subject,
        teacher: p.teacher.clone(),
    };
    insert::insert_lesson(conn, &lesson_insert)?;
    append_log(log_user, TBL_LESSONS as u8, OP_INSERT, 0)?;

    let row = fetch_lesson(
        conn,
        &p.school,
        p.year,
        p.term as i16,
        p.grade as i16,
        p.stream as i16,
        p.date,
        p.subject as i16,
        &p.teacher,
    )?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_LESSONS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Lesson((&row).into())),
        },
    )]))
}

fn handle_delete_lesson(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteLessonPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        p.school, p.year, p.term, p.grade, p.stream, p.date, p.subject, p.teacher
    );

    delete::delete_lesson(conn, &row_key)?;
    append_log(log_user, TBL_LESSONS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_LESSONS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_LESSONS,
        row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Exams
// ---------------------------------------------------------------------------

fn handle_create_exam(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateExamPayload = decode(payload)?;
    let log_user = Id::system();

    let exam_insert = ExamInsert {
        id: p.id.clone(),
        school: p.school.clone(),
        name: p.name.clone(),
        year: p.year,
        term: p.term,
        personalized: p.personalized,
        r#type: p.r#type,
        start: p.start,
        end: p.end,
        teacher: p.teacher.clone(),
    };
    insert::insert_exam(conn, &exam_insert)?;
    append_log(log_user, TBL_EXAMS as u8, OP_INSERT, 0)?;

    let row = fetch_exam(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_EXAMS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Exam((&row).into())),
        },
    )]))
}

fn handle_update_exam(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateExamPayload = decode(payload)?;
    let log_user = Id::system();

    update::update_exam(conn, &p.id, &p)?;
    append_log(log_user, TBL_EXAMS as u8, OP_UPDATE, 0)?;

    let row = fetch_exam(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_EXAMS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Exam((&row).into())),
        },
    )]))
}

fn handle_delete_exam(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteExamPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = p.id.clone();

    delete::delete_exam(conn, &row_key)?;
    append_log(log_user, TBL_EXAMS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_EXAMS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_EXAMS, row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Papers
// ---------------------------------------------------------------------------

fn handle_create_paper(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreatePaperPayload = decode(payload)?;
    let log_user = Id::system();

    let paper_insert = PaperInsert {
        school: p.school.clone(),
        exam: p.exam.clone(),
        subject: p.subject,
        paper: p.paper,
        topic: p.topic,
        invigilator: p.invigilator.clone(),
        start: p.start,
        end: p.end,
        status: 0, // default status
        grade: p.grade,
        stream: p.stream,
    };
    insert::insert_paper(conn, &paper_insert)?;
    append_log(log_user, TBL_PAPERS as u8, OP_INSERT, 0)?;

    let row = fetch_paper(
        conn,
        &p.school,
        &p.exam,
        p.subject as i32,
        p.paper.map(|v| v as i16),
        p.grade as i16,
        p.stream.map(|v| v as i16),
    )?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_PAPERS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Paper((&row).into())),
        },
    )]))
}

fn handle_update_paper(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdatePaperPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!(
        "{}|{}|{}|{}|{}|{}",
        p.school,
        p.exam,
        p.subject,
        p.paper.map(|v| v.to_string()).unwrap_or_default(),
        p.grade,
        p.stream.map(|v| v.to_string()).unwrap_or_default()
    );

    update::update_paper(conn, &row_key, &p)?;
    append_log(log_user, TBL_PAPERS as u8, OP_UPDATE, 0)?;

    let row = fetch_paper(
        conn,
        &p.school,
        &p.exam,
        p.subject as i32,
        p.paper.map(|v| v as i16),
        p.grade as i16,
        p.stream.map(|v| v as i16),
    )?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_PAPERS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Paper((&row).into())),
        },
    )]))
}

fn handle_delete_paper(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeletePaperPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!(
        "{}|{}|{}|{}|{}|{}",
        p.school,
        p.exam,
        p.subject,
        p.paper.map(|v| v.to_string()).unwrap_or_default(),
        p.grade,
        p.stream.map(|v| v.to_string()).unwrap_or_default()
    );

    delete::delete_paper(conn, &row_key)?;
    append_log(log_user, TBL_PAPERS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_PAPERS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_PAPERS, row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Grades
// ---------------------------------------------------------------------------

fn handle_mark_grades(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: MarkGradesPayload = decode(payload)?;
    let log_user = Id::system();
    let mut rows = Vec::new();

    let paper_i16 = p.paper.map(|v| v as i16);

    for rec in &p.records {
        let grade_insert = GradeInsert {
            school: p.school.clone(),
            exam: p.exam.clone(),
            student: rec.student,
            subject: p.subject,
            paper: p.paper,
            score: rec.score,
            total: rec.total,
        };
        let row_key = format!(
            "{}|{}|{}|{}|{}",
            p.school,
            p.exam,
            rec.student,
            p.subject,
            p.paper.map(|v| v.to_string()).unwrap_or_default()
        );
        // Upsert: try insert, on conflict update
        let inserted = insert::insert_grade(conn, &grade_insert);
        if inserted.is_err() {
            // Row exists — update score/total
            let update_payload = UpdateGradePayload {
                school: p.school.clone(),
                exam: p.exam.clone(),
                student: rec.student,
                subject: p.subject,
                paper: p.paper,
                score: Some(rec.score),
                total: Some(rec.total),
            };
            update::update_grade(conn, &row_key, &update_payload)?;
            append_log(log_user, TBL_GRADES as u8, OP_UPDATE, 0)?;
        } else {
            append_log(log_user, TBL_GRADES as u8, OP_INSERT, 0)?;
        }

        let row = fetch_grade(
            conn,
            &p.school,
            &p.exam,
            rec.student,
            p.subject as i32,
            paper_i16,
        )?;
        rows.push(upsert_row(
            TBL_GRADES,
            row.row_key(),
            InsertData {
                row: Some(insert_data::Row::Grade((&row).into())),
            },
        ));
    }

    Ok(ActionResult::with_rows(rows))
}

fn handle_update_grade(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateGradePayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!(
        "{}|{}|{}|{}|{}",
        p.school,
        p.exam,
        p.student,
        p.subject,
        p.paper.map(|v| v.to_string()).unwrap_or_default()
    );

    update::update_grade(conn, &row_key, &p)?;
    append_log(log_user, TBL_GRADES as u8, OP_UPDATE, 0)?;

    let row = fetch_grade(
        conn,
        &p.school,
        &p.exam,
        p.student,
        p.subject as i32,
        p.paper.map(|v| v as i16),
    )?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_GRADES,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Grade((&row).into())),
        },
    )]))
}

fn handle_delete_grade(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteGradePayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!(
        "{}|{}|{}|{}|{}",
        p.school,
        p.exam,
        p.student,
        p.subject,
        p.paper.map(|v| v.to_string()).unwrap_or_default()
    );

    delete::delete_grade(conn, &row_key)?;
    append_log(log_user, TBL_GRADES as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_GRADES as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_GRADES, row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Mastery
// ---------------------------------------------------------------------------

fn handle_update_mastery(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateMasteryPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}|{}|{}", p.school, p.student, p.subject, p.topic);

    // Upsert: try insert, on conflict update
    let mastery_insert = MasteryInsert {
        school: p.school.clone(),
        student: p.student,
        subject: p.subject,
        topic: p.topic,
        score: p.score,
    };
    let inserted = insert::insert_mastery(conn, &mastery_insert);
    if inserted.is_err() {
        update::update_mastery(conn, &row_key, &p)?;
        append_log(log_user, TBL_MASTERY as u8, OP_UPDATE, 0)?;
    } else {
        append_log(log_user, TBL_MASTERY as u8, OP_INSERT, 0)?;
    }

    let row = fetch_mastery(conn, &p.school, p.student, p.subject as i32, p.topic as i32)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_MASTERY,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Mastery((&row).into())),
        },
    )]))
}

// ---------------------------------------------------------------------------
// Fees
// ---------------------------------------------------------------------------

fn handle_create_fee(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateFeePayload = decode(payload)?;
    let log_user = Id::system();

    let fee_insert = FeeInsert {
        id: p.id.clone(),
        school: p.school.clone(),
        year: p.year,
        term: p.term,
        grade: p.grade,
        title: p.title.clone(),
        description: p.description.clone(),
        amount: p.amount,
        mandatory: p.mandatory,
        due: p.due,
    };
    insert::insert_fee(conn, &fee_insert)?;
    append_log(log_user, TBL_FEES as u8, OP_INSERT, 0)?;

    let row = fetch_fee(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_FEES,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Fee((&row).into())),
        },
    )]))
}

fn handle_update_fee(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateFeePayload = decode(payload)?;
    let log_user = Id::system();

    update::update_fee(conn, &p.id, &p)?;
    append_log(log_user, TBL_FEES as u8, OP_UPDATE, 0)?;

    let row = fetch_fee(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_FEES,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Fee((&row).into())),
        },
    )]))
}

fn handle_delete_fee(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteFeePayload = decode(payload)?;
    let log_user = Id::system();

    delete::delete_fee(conn, &p.id)?;
    append_log(log_user, TBL_FEES as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_FEES as u8, &p.id)?;

    Ok(ActionResult::with_rows(vec![delete_row(TBL_FEES, p.id)]))
}

// ---------------------------------------------------------------------------
// Invoices
// ---------------------------------------------------------------------------

fn handle_create_invoice(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateInvoicePayload = decode(payload)?;
    let log_user = Id::system();

    let invoice_insert = InvoiceInsert {
        id: p.id.clone(),
        school: p.school.clone(),
        year: p.year,
        term: p.term,
        fee: p.fee.clone(),
        description: p.description.clone(),
        student: p.student,
        amount: p.amount,
        status: 0, // default: unpaid
        due: p.due,
    };
    insert::insert_invoice(conn, &invoice_insert)?;
    append_log(log_user, TBL_INVOICES as u8, OP_INSERT, 0)?;

    let row = fetch_invoice(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_INVOICES,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Invoice((&row).into())),
        },
    )]))
}

fn handle_update_invoice(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateInvoicePayload = decode(payload)?;
    let log_user = Id::system();

    update::update_invoice(conn, &p.id, &p)?;
    append_log(log_user, TBL_INVOICES as u8, OP_UPDATE, 0)?;

    let row = fetch_invoice(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_INVOICES,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Invoice((&row).into())),
        },
    )]))
}

fn handle_delete_invoice(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteInvoicePayload = decode(payload)?;
    let log_user = Id::system();

    delete::delete_invoice(conn, &p.id)?;
    append_log(log_user, TBL_INVOICES as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_INVOICES as u8, &p.id)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_INVOICES,
        p.id,
    )]))
}

// ---------------------------------------------------------------------------
// Payments
// ---------------------------------------------------------------------------

fn handle_create_payment(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreatePaymentPayload = decode(payload)?;
    let log_user = Id::system();

    let payment_insert = PaymentInsert {
        id: p.id.clone(),
        invoice: p.invoice.clone(),
        school: p.school.clone(),
        student: p.student,
        amount: p.amount,
        method: p.method,
        reference: p.reference.clone(),
        recorder: p.recorder.clone(),
        date: p.date,
    };
    insert::insert_payment(conn, &payment_insert)?;
    append_log(log_user, TBL_PAYMENTS as u8, OP_INSERT, 0)?;

    let row = fetch_payment(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_PAYMENTS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Payment((&row).into())),
        },
    )]))
}

fn handle_update_payment(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdatePaymentPayload = decode(payload)?;
    let log_user = Id::system();

    update::update_payment(conn, &p.id, &p)?;
    append_log(log_user, TBL_PAYMENTS as u8, OP_UPDATE, 0)?;

    let row = fetch_payment(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_PAYMENTS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Payment((&row).into())),
        },
    )]))
}

fn handle_delete_payment(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeletePaymentPayload = decode(payload)?;
    let log_user = Id::system();

    delete::delete_payment(conn, &p.id)?;
    append_log(log_user, TBL_PAYMENTS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_PAYMENTS as u8, &p.id)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_PAYMENTS,
        p.id,
    )]))
}

fn handle_approve_payment(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: ApprovePaymentPayload = decode(payload)?;
    let log_user = Id::system();

    // Approve = update payment status. We reuse update_payment with a status field.
    // The UpdatePaymentPayload doesn't have a status field, so we do a raw SQL update.
    let now = chrono::Utc::now().timestamp();
    sql_query("UPDATE payments SET updated = ? WHERE id = ?")
        .bind::<diesel::sql_types::BigInt, _>(now)
        .bind::<Text, _>(&p.id)
        .execute(conn)?;
    append_log(log_user, TBL_PAYMENTS as u8, OP_UPDATE, 0)?;

    let row = fetch_payment(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_PAYMENTS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Payment((&row).into())),
        },
    )]))
}

// ---------------------------------------------------------------------------
// Announcements
// ---------------------------------------------------------------------------

fn handle_create_announcement(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateAnnouncementPayload = decode(payload)?;
    let log_user = Id::system();

    let announcement_insert = AnnouncementInsert {
        id: p.id.clone(),
        school: p.school.clone(),
        title: p.title.clone(),
        content: p.content.clone(),
        grade: p.grade,
        stream: p.stream,
        audience: p.audience,
        author: p.author.clone(),
    };
    insert::insert_announcement(conn, &announcement_insert)?;
    append_log(log_user, TBL_ANNOUNCEMENTS as u8, OP_INSERT, 0)?;

    let row = fetch_announcement(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_ANNOUNCEMENTS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Announcement((&row).into())),
        },
    )]))
}

fn handle_update_announcement(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateAnnouncementPayload = decode(payload)?;
    let log_user = Id::system();

    update::update_announcement(conn, &p.id, &p)?;
    append_log(log_user, TBL_ANNOUNCEMENTS as u8, OP_UPDATE, 0)?;

    let row = fetch_announcement(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_ANNOUNCEMENTS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Announcement((&row).into())),
        },
    )]))
}

fn handle_delete_announcement(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteAnnouncementPayload = decode(payload)?;
    let log_user = Id::system();

    delete::delete_announcement(conn, &p.id)?;
    append_log(log_user, TBL_ANNOUNCEMENTS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_ANNOUNCEMENTS as u8, &p.id)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_ANNOUNCEMENTS,
        p.id,
    )]))
}

// ---------------------------------------------------------------------------
// Roles
// ---------------------------------------------------------------------------

fn handle_create_role(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateRolePayload = decode(payload)?;
    let log_user = Id::system();

    let role_insert = RoleInsert {
        id: p.id.clone(),
        school: p.school.clone(),
        name: p.name.clone(),
        description: p.description.clone(),
        permissions: p.permissions.clone(),
    };
    insert::insert_role(conn, &role_insert)?;
    append_log(log_user, TBL_ROLES as u8, OP_INSERT, 0)?;

    let row = fetch_role(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_ROLES,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Role((&row).into())),
        },
    )]))
}

fn handle_update_role(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateRolePayload = decode(payload)?;
    let log_user = Id::system();

    update::update_role(conn, &p.id, &p)?;
    append_log(log_user, TBL_ROLES as u8, OP_UPDATE, 0)?;

    let row = fetch_role(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_ROLES,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Role((&row).into())),
        },
    )]))
}

fn handle_delete_role(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteRolePayload = decode(payload)?;
    let log_user = Id::system();

    delete::delete_role(conn, &p.id)?;
    append_log(log_user, TBL_ROLES as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_ROLES as u8, &p.id)?;

    Ok(ActionResult::with_rows(vec![delete_row(TBL_ROLES, p.id)]))
}

fn handle_assign_role(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: AssignRolePayload = decode(payload)?;
    let log_user = Id::system();

    let scope_insert = ScopeInsert {
        school: p.school.clone(),
        user: p.user.clone(),
        role: p.role.clone(),
    };
    insert::insert_scope(conn, &scope_insert)?;
    append_log(log_user, TBL_SCOPES as u8, OP_INSERT, 0)?;

    let row = fetch_scope(conn, p.school.as_deref(), &p.user, &p.role)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_SCOPES,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Scope((&row).into())),
        },
    )]))
}

fn handle_unassign_role(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UnassignRolePayload = decode(payload)?;
    let log_user = Id::system();

    let row_key = format!(
        "{}|{}|{}",
        p.school.as_deref().unwrap_or(""),
        p.user,
        p.role
    );

    delete::delete_scope(conn, &row_key)?;
    append_log(log_user, TBL_SCOPES as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_SCOPES as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_SCOPES, row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------------

fn handle_update_user(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateUserPayload = decode(payload)?;
    let log_user = Id::system();

    update::update_user(conn, &p.id, &p)?;
    append_log(log_user, TBL_USERS as u8, OP_UPDATE, 0)?;

    let row = fetch_user(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_USERS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::User((&row).into())),
        },
    )]))
}

fn handle_delete_user(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteUserPayload = decode(payload)?;
    let log_user = Id::system();

    // Soft-delete: set status to deleted (status = 2).
    let soft_delete = UpdateUserPayload {
        id: p.id.clone(),
        phone: None,
        email: None,
        name: None,
        level: None,
        status: Some(2), // Deleted
    };
    update::update_user(conn, &p.id, &soft_delete)?;
    append_log(log_user, TBL_USERS as u8, OP_UPDATE, 0)?;

    let row = fetch_user(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_USERS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::User((&row).into())),
        },
    )]))
}

// ---------------------------------------------------------------------------
// Plans
// ---------------------------------------------------------------------------

fn handle_create_plan(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreatePlanPayload = decode(payload)?;
    let log_user = Id::system();

    let plan_insert = PlanInsert {
        id: p.id.clone(),
        name: p.name.clone(),
        description: p.description.clone(),
        amount: p.amount,
        levels: p.levels,
        status: 0, // Active
        features: p.features.clone(),
    };
    insert::insert_plan(conn, &plan_insert)?;
    append_log(log_user, TBL_PLANS as u8, OP_INSERT, 0)?;

    let row = fetch_plan(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_PLANS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Plan((&row).into())),
        },
    )]))
}

fn handle_update_plan(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdatePlanPayload = decode(payload)?;
    let log_user = Id::system();

    update::update_plan(conn, &p.id, &p)?;
    append_log(log_user, TBL_PLANS as u8, OP_UPDATE, 0)?;

    let row = fetch_plan(conn, &p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_PLANS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Plan((&row).into())),
        },
    )]))
}

fn handle_delete_plan(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeletePlanPayload = decode(payload)?;
    let log_user = Id::system();

    // Hard-delete for plans.
    super::delete::delete_plan(conn, &p.id)?;
    append_log(log_user, TBL_PLANS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_PLANS as u8, &p.id)?;

    Ok(ActionResult::with_rows(vec![delete_row(TBL_PLANS, p.id)]))
}

// ---------------------------------------------------------------------------
// AI usage
// ---------------------------------------------------------------------------

fn handle_update_ai_usage(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateAiUsagePayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}|{}|{}", p.school, p.student, p.year, p.term);

    // Upsert: try insert, on conflict update
    let ai_insert = AiUsageInsert {
        school: p.school.clone(),
        student: p.student,
        year: p.year,
        term: p.term,
        allocated: p.allocated.unwrap_or(0),
        used: p.used.unwrap_or(0),
    };
    let inserted = insert::insert_ai_usage(conn, &ai_insert);
    if inserted.is_err() {
        update::update_ai_usage(conn, &row_key, &p)?;
        append_log(log_user, TBL_AI_USAGE as u8, OP_UPDATE, 0)?;
    } else {
        append_log(log_user, TBL_AI_USAGE as u8, OP_INSERT, 0)?;
    }

    let row = fetch_ai_usage(conn, &p.school, p.student, p.year, p.term as i16)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_AI_USAGE,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::AiUsage((&row).into())),
        },
    )]))
}

// ---------------------------------------------------------------------------
// Subscriptions
// ---------------------------------------------------------------------------

fn handle_create_subscription(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateSubscriptionPayload = decode(payload)?;
    let log_user = Id::system();

    let sub_insert = SubscriptionInsert {
        school: p.school.clone(),
        plan: p.plan.clone(),
        year: p.year,
        term: p.term,
        student: p.student,
        invoice: p.invoice.clone(),
        discount: p.discount,
        status: 0, // default: active
    };
    insert::insert_subscription(conn, &sub_insert)?;
    append_log(log_user, TBL_SUBSCRIPTIONS as u8, OP_INSERT, 0)?;

    let row = fetch_subscription(conn, &p.school, &p.plan, p.year, p.term as i16, p.student)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_SUBSCRIPTIONS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Subscription((&row).into())),
        },
    )]))
}

fn handle_update_subscription(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateSubscriptionPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!(
        "{}|{}|{}|{}|{}",
        p.school, p.plan, p.year, p.term, p.student
    );

    update::update_subscription(conn, &row_key, &p)?;
    append_log(log_user, TBL_SUBSCRIPTIONS as u8, OP_UPDATE, 0)?;

    let row = fetch_subscription(conn, &p.school, &p.plan, p.year, p.term as i16, p.student)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_SUBSCRIPTIONS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Subscription((&row).into())),
        },
    )]))
}

fn handle_delete_subscription(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteSubscriptionPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!(
        "{}|{}|{}|{}|{}",
        p.school, p.plan, p.year, p.term, p.student
    );

    delete::delete_subscription(conn, &row_key)?;
    append_log(log_user, TBL_SUBSCRIPTIONS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_SUBSCRIPTIONS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_SUBSCRIPTIONS,
        row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Discounts
// ---------------------------------------------------------------------------

fn handle_create_discount(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateDiscountPayload = decode(payload)?;
    let log_user = Id::system();

    let discount_insert = DiscountInsert {
        school: p.school.clone(),
        plan: p.plan.clone(),
        year: p.year,
        term: p.term,
        grade: p.grade,
        amount: p.amount,
        unit: p.unit,
    };
    insert::insert_discount(conn, &discount_insert)?;
    append_log(log_user, TBL_DISCOUNTS as u8, OP_INSERT, 0)?;

    let row = fetch_discount(
        conn,
        &p.school,
        &p.plan,
        p.year,
        p.term as i16,
        p.grade as i16,
    )?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_DISCOUNTS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Discount((&row).into())),
        },
    )]))
}

fn handle_update_discount(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateDiscountPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}|{}|{}|{}", p.school, p.plan, p.year, p.term, p.grade);

    update::update_discount(conn, &row_key, &p)?;
    append_log(log_user, TBL_DISCOUNTS as u8, OP_UPDATE, 0)?;

    let row = fetch_discount(
        conn,
        &p.school,
        &p.plan,
        p.year,
        p.term as i16,
        p.grade as i16,
    )?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_DISCOUNTS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Discount((&row).into())),
        },
    )]))
}

fn handle_delete_discount(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteDiscountPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}|{}|{}|{}", p.school, p.plan, p.year, p.term, p.grade);

    delete::delete_discount(conn, &row_key)?;
    append_log(log_user, TBL_DISCOUNTS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_DISCOUNTS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_DISCOUNTS,
        row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Subjects (global catalog)
// ---------------------------------------------------------------------------

fn handle_create_subject(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateSubjectPayload = decode(payload)?;
    let log_user = Id::system();
    let now = chrono::Utc::now().timestamp();

    diesel::sql_query(
        "INSERT INTO subjects (name, curriculum, created, updated) VALUES (?, ?, ?, ?)",
    )
    .bind::<diesel::sql_types::Text, _>(&p.name)
    .bind::<diesel::sql_types::SmallInt, _>(p.curriculum as i16)
    .bind::<diesel::sql_types::BigInt, _>(now)
    .bind::<diesel::sql_types::BigInt, _>(now)
    .execute(conn)
    .map_err(|e| {
        tracing::error!("insert_subject failed: {e}");
        Error::Internal
    })?;

    append_log(log_user, TBL_SUBJECT_CATALOG as u8, OP_INSERT, 0)?;

    // Fetch back by name+curriculum to get the assigned id
    let row = sql_query(
        "SELECT id, name, curriculum, created, updated FROM subjects WHERE name = ? AND curriculum = ? ORDER BY id DESC LIMIT 1",
    )
    .bind::<diesel::sql_types::Text, _>(&p.name)
    .bind::<diesel::sql_types::SmallInt, _>(p.curriculum as i16)
    .load::<SubjectCatalogRow>(conn)
    .map_err(|e| { tracing::error!("fetch after insert_subject failed: {e}"); Error::Internal })?
    .into_iter()
    .next()
    .ok_or_else(|| { tracing::error!("subject not found after insert"); Error::Internal })?;

    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_SUBJECT_CATALOG,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::SubjectCatalog((&row).into())),
        },
    )]))
}

fn handle_update_subject(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateSubjectPayload = decode(payload)?;
    let log_user = Id::system();

    update::update_subject_catalog(conn, p.id, &p)?;
    append_log(log_user, TBL_SUBJECT_CATALOG as u8, OP_UPDATE, 0)?;

    let row = fetch_subject_catalog(conn, p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_SUBJECT_CATALOG,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::SubjectCatalog((&row).into())),
        },
    )]))
}

fn handle_delete_subject(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteSubjectPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = p.id.to_string();

    delete::delete_subject_catalog(conn, p.id)?;
    append_log(log_user, TBL_SUBJECT_CATALOG as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_SUBJECT_CATALOG as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_SUBJECT_CATALOG,
        row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Topics (global catalog)
// ---------------------------------------------------------------------------

fn handle_create_topic(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateTopicPayload = decode(payload)?;
    let log_user = Id::system();
    let now = chrono::Utc::now().timestamp();

    diesel::sql_query(
        "INSERT INTO topics (subject, grade, name, created, updated) VALUES (?, ?, ?, ?, ?)",
    )
    .bind::<diesel::sql_types::Integer, _>(p.subject)
    .bind::<diesel::sql_types::SmallInt, _>(p.grade as i16)
    .bind::<diesel::sql_types::Text, _>(&p.name)
    .bind::<diesel::sql_types::BigInt, _>(now)
    .bind::<diesel::sql_types::BigInt, _>(now)
    .execute(conn)
    .map_err(|e| {
        tracing::error!("insert_topic failed: {e}");
        Error::Internal
    })?;

    append_log(log_user, TBL_TOPICS as u8, OP_INSERT, 0)?;

    let row = sql_query(
        "SELECT id, subject, grade, name, created, updated FROM topics WHERE subject = ? AND grade = ? AND name = ? ORDER BY id DESC LIMIT 1",
    )
    .bind::<diesel::sql_types::Integer, _>(p.subject)
    .bind::<diesel::sql_types::SmallInt, _>(p.grade as i16)
    .bind::<diesel::sql_types::Text, _>(&p.name)
    .load::<TopicRow>(conn)
    .map_err(|e| { tracing::error!("fetch after insert_topic failed: {e}"); Error::Internal })?
    .into_iter()
    .next()
    .ok_or_else(|| { tracing::error!("topic not found after insert"); Error::Internal })?;

    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_TOPICS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Topic((&row).into())),
        },
    )]))
}

fn handle_update_topic(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateTopicPayload = decode(payload)?;
    let log_user = Id::system();

    update::update_topic(conn, p.id, &p)?;
    append_log(log_user, TBL_TOPICS as u8, OP_UPDATE, 0)?;

    let row = fetch_topic(conn, p.id)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_TOPICS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Topic((&row).into())),
        },
    )]))
}

fn handle_delete_topic(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteTopicPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = p.id.to_string();

    delete::delete_topic(conn, p.id)?;
    append_log(log_user, TBL_TOPICS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_TOPICS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_TOPICS, row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Streams
// ---------------------------------------------------------------------------

fn handle_create_stream(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateStreamPayload = decode(payload)?;
    let log_user = Id::system();

    let stream_insert = StreamInsert {
        school: p.school.clone(),
        grade: p.grade,
        stream: p.stream,
        name: p.name.clone(),
    };
    insert::insert_stream(conn, &stream_insert)?;
    append_log(log_user, TBL_STREAMS as u8, OP_INSERT, 0)?;

    let row = fetch_stream(conn, &p.school, p.grade as i16, p.stream as i16)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_STREAMS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Stream((&row).into())),
        },
    )]))
}

fn handle_update_stream(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateStreamPayload = decode(payload)?;
    let log_user = Id::system();

    update::update_stream(conn, &p)?;
    append_log(log_user, TBL_STREAMS as u8, OP_UPDATE, 0)?;

    let row = fetch_stream(conn, &p.school, p.grade as i16, p.stream as i16)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_STREAMS,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Stream((&row).into())),
        },
    )]))
}

fn handle_delete_stream(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteStreamPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = format!("{}|{}|{}", p.school, p.grade, p.stream);

    delete::delete_stream(conn, &p.school, p.grade as i16, p.stream as i16)?;
    append_log(log_user, TBL_STREAMS as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_STREAMS as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_STREAMS,
        row_key,
    )]))
}

// ---------------------------------------------------------------------------
// Mpesa
// ---------------------------------------------------------------------------

fn handle_create_mpesa(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: CreateMpesaPayload = decode(payload)?;
    let log_user = Id::system();

    let mpesa_insert = MpesaInsert {
        school: p.school.clone(),
        consumer_key: p.consumer_key.clone(),
        consumer_secret: p.consumer_secret.clone(),
        passkey: p.passkey.clone(),
        shortcode: p.shortcode.clone(),
        env: p.env,
    };
    insert::insert_mpesa(conn, &mpesa_insert)?;
    append_log(log_user, TBL_MPESA as u8, OP_INSERT, 0)?;

    let row = fetch_mpesa(conn, &p.school)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_MPESA,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Mpesa((&row).into())),
        },
    )]))
}

fn handle_update_mpesa(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: UpdateMpesaPayload = decode(payload)?;
    let log_user = Id::system();

    update::update_mpesa(conn, &p)?;
    append_log(log_user, TBL_MPESA as u8, OP_UPDATE, 0)?;

    let row = fetch_mpesa(conn, &p.school)?;
    Ok(ActionResult::with_rows(vec![upsert_row(
        TBL_MPESA,
        row.row_key(),
        InsertData {
            row: Some(insert_data::Row::Mpesa((&row).into())),
        },
    )]))
}

fn handle_delete_mpesa(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteMpesaPayload = decode(payload)?;
    let log_user = Id::system();
    let row_key = p.school.clone();

    delete::delete_mpesa(conn, &p.school)?;
    append_log(log_user, TBL_MPESA as u8, OP_DELETE, 0)?;
    append_delete_log(TBL_MPESA as u8, &row_key)?;

    Ok(ActionResult::with_rows(vec![delete_row(
        TBL_MPESA, row_key,
    )]))
}

// ---------------------------------------------------------------------------
// File sync: Scheme pages
// ---------------------------------------------------------------------------

/// Replace/set all scheme pages for a (school, exam, subject, paper) combination.
///
/// 1. Delete any existing scheme pages for that paper (and log each delete).
/// 2. Insert `count` new page rows with presigned S3 keys.
/// 3. Return the new rows plus presigned PUT URLs so the originator can upload.
fn handle_upload_scheme(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    use crate::config::storage::sign;

    let p: UploadSchemePayload = decode(payload)?;
    let log_user = Id::system();
    let paper_i16 = p.paper.map(|v| v as i16);
    // Use 0 as the display sentinel when paper IS NULL (single-paper subject).
    let paper_display = p.paper.unwrap_or(0);

    // 1. Delete existing pages and log each one.
    let existing_pages =
        delete::delete_scheme_pages(conn, &p.school, &p.exam, p.subject, paper_i16)?;
    for page in &existing_pages {
        let row_key = format!(
            "{}|{}|{}|{}|{}",
            p.school,
            p.exam,
            p.subject,
            p.paper.map(|v| v.to_string()).unwrap_or_default(),
            page
        );
        append_log(log_user, TBL_SCHEME_PAGES as u8, OP_DELETE, 0)?;
        append_delete_log(TBL_SCHEME_PAGES as u8, &row_key)?;
    }

    let now = chrono::Utc::now().timestamp();
    let mut rows = Vec::new();
    let mut file_urls = Vec::new();

    // 2. Insert new pages and generate presigned PUT URLs.
    for page_idx in 0..p.count {
        let page = page_idx as i16;
        // S3 key — matches sign::scheme_image path convention.
        let s3_key = format!(
            "schools/{}/exams/{}/papers/{}_{}/scheme/{}",
            p.school, p.exam, p.subject, paper_display, page_idx
        );

        insert::insert_scheme_page(
            conn, &p.school, &p.exam, p.subject, paper_i16, page, &s3_key, now,
        )?;
        append_log(log_user, TBL_SCHEME_PAGES as u8, OP_INSERT, 0)?;

        let row_key = format!(
            "{}|{}|{}|{}|{}",
            p.school,
            p.exam,
            p.subject,
            p.paper.map(|v| v.to_string()).unwrap_or_default(),
            page
        );

        rows.push(upsert_row(
            TBL_SCHEME_PAGES,
            row_key,
            InsertData {
                row: Some(insert_data::Row::SchemePage(SchemePageInsert {
                    school: p.school.clone(),
                    exam: p.exam.clone(),
                    subject: p.subject,
                    paper: p.paper,
                    page: page_idx,
                    key: s3_key.clone(),
                    created: now,
                })),
            },
        ));

        // Presigned PUT URL — valid for PUT_TTL (1 hour).
        let put_url = sign::url(&s3_key, sign::PUT_TTL, true);
        // Local client path (relative to appDir).
        let local_path = format!(
            "submissions/{}/{}/{}_{}/scheme/{}.jpg",
            p.school, p.exam, p.subject, paper_display, page_idx
        );
        file_urls.push(FileUrl {
            path: local_path,
            put_url: Some(put_url),
            get_url: None,
            expiry: now + sign::PUT_TTL as i64,
        });
    }

    Ok(ActionResult::with_rows_and_urls(rows, file_urls))
}

/// Remove all scheme pages for a (school, exam, subject, paper) combination.
fn handle_delete_scheme(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteSchemePayload = decode(payload)?;
    let log_user = Id::system();
    let paper_i16 = p.paper.map(|v| v as i16);

    let existing_pages =
        delete::delete_scheme_pages(conn, &p.school, &p.exam, p.subject, paper_i16)?;
    let mut rows = Vec::new();

    for page in existing_pages {
        let row_key = format!(
            "{}|{}|{}|{}|{}",
            p.school,
            p.exam,
            p.subject,
            p.paper.map(|v| v.to_string()).unwrap_or_default(),
            page
        );
        append_log(log_user, TBL_SCHEME_PAGES as u8, OP_DELETE, 0)?;
        append_delete_log(TBL_SCHEME_PAGES as u8, &row_key)?;
        rows.push(delete_row(TBL_SCHEME_PAGES, row_key));
    }

    Ok(ActionResult::with_rows(rows))
}

// ---------------------------------------------------------------------------
// File sync: Answer sheet pages
// ---------------------------------------------------------------------------

/// Replace/set all answer sheet pages for a (school, exam, student, subject, paper) combination.
///
/// 1. Delete any existing answer pages for that entry (and log each delete).
/// 2. Insert `count` new page rows with presigned S3 keys.
/// 3. Return the new rows plus presigned PUT URLs so the originator can upload.
fn handle_upload_answer_sheet(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    use crate::config::storage::sign;

    let p: UploadAnswerSheetPayload = decode(payload)?;
    let log_user = Id::system();
    let paper_i16 = p.paper.map(|v| v as i16);
    let paper_display = p.paper.unwrap_or(0);

    // 1. Delete existing pages and log each one.
    let existing_pages =
        delete::delete_answer_pages(conn, &p.school, &p.exam, p.student, p.subject, paper_i16)?;
    for page in &existing_pages {
        let row_key = format!(
            "{}|{}|{}|{}|{}|{}",
            p.school,
            p.exam,
            p.student,
            p.subject,
            p.paper.map(|v| v.to_string()).unwrap_or_default(),
            page
        );
        append_log(log_user, TBL_ANSWER_PAGES as u8, OP_DELETE, 0)?;
        append_delete_log(TBL_ANSWER_PAGES as u8, &row_key)?;
    }

    let now = chrono::Utc::now().timestamp();
    let mut rows = Vec::new();
    let mut file_urls = Vec::new();

    // 2. Insert new pages and generate presigned PUT URLs.
    for page_idx in 0..p.count {
        let page = page_idx as i16;
        // S3 key — matches sign::answer_sheet path convention.
        let s3_key = format!(
            "schools/{}/exams/{}/papers/{}_{}/students/{}/{}",
            p.school, p.exam, p.subject, paper_display, p.student, page_idx
        );

        insert::insert_answer_page(
            conn, &p.school, &p.exam, p.student, p.subject, paper_i16, page, &s3_key, now,
        )?;
        append_log(log_user, TBL_ANSWER_PAGES as u8, OP_INSERT, 0)?;

        let row_key = format!(
            "{}|{}|{}|{}|{}|{}",
            p.school,
            p.exam,
            p.student,
            p.subject,
            p.paper.map(|v| v.to_string()).unwrap_or_default(),
            page
        );

        rows.push(upsert_row(
            TBL_ANSWER_PAGES,
            row_key,
            InsertData {
                row: Some(insert_data::Row::AnswerPage(AnswerPageInsert {
                    school: p.school.clone(),
                    exam: p.exam.clone(),
                    student: p.student,
                    subject: p.subject,
                    paper: p.paper,
                    page: page_idx,
                    key: s3_key.clone(),
                    created: now,
                })),
            },
        ));

        // Presigned PUT URL — valid for PUT_TTL (1 hour).
        let put_url = sign::url(&s3_key, sign::PUT_TTL, true);
        // Local client path (relative to appDir).
        let local_path = format!(
            "submissions/{}/{}/{}_{}/{}/{}.jpg",
            p.school, p.exam, p.subject, paper_display, p.student, page_idx
        );
        file_urls.push(FileUrl {
            path: local_path,
            put_url: Some(put_url),
            get_url: None,
            expiry: now + sign::PUT_TTL as i64,
        });
    }

    Ok(ActionResult::with_rows_and_urls(rows, file_urls))
}

/// Remove all answer sheet pages for a (school, exam, student, subject, paper) combination.
fn handle_delete_answer_sheet(conn: &mut Conn, payload: &[u8]) -> Result<ActionResult> {
    let p: DeleteAnswerSheetPayload = decode(payload)?;
    let log_user = Id::system();
    let paper_i16 = p.paper.map(|v| v as i16);

    let existing_pages =
        delete::delete_answer_pages(conn, &p.school, &p.exam, p.student, p.subject, paper_i16)?;
    let mut rows = Vec::new();

    for page in existing_pages {
        let row_key = format!(
            "{}|{}|{}|{}|{}|{}",
            p.school,
            p.exam,
            p.student,
            p.subject,
            p.paper.map(|v| v.to_string()).unwrap_or_default(),
            page
        );
        append_log(log_user, TBL_ANSWER_PAGES as u8, OP_DELETE, 0)?;
        append_delete_log(TBL_ANSWER_PAGES as u8, &row_key)?;
        rows.push(delete_row(TBL_ANSWER_PAGES, row_key));
    }

    Ok(ActionResult::with_rows(rows))
}
