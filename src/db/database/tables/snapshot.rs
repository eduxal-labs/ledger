use crate::proto::services::sync::{InsertData, insert_data};
use crate::types::error::{Error, Result};
use crate::types::id::Id;
use diesel::RunQueryDsl;
use diesel::SqliteConnection as Conn;
use diesel::sql_types::BigInt;
use tracing::error;

use super::rows::*;

// Table constants mirroring LogTable in services/sync.rs.
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
const TBL_AIUSAGE: i32 = 24;
const TBL_SUBJECT_CATALOG: i32 = 31;
const TBL_TOPICS: i32 = 32;
const TBL_STREAMS: i32 = 33;
const TBL_MPESA: i32 = 34;
const TBL_SCHEME_PAGES: i32 = 36;
const TBL_ANSWER_PAGES: i32 = 37;
const TBL_ROLES: i32 = 26;
const TBL_SCOPES: i32 = 27;
const TBL_PLANS: i32 = 28;
const TBL_SUBSCRIPTIONS: i32 = 29;
const TBL_DISCOUNTS: i32 = 30;

/// A single row snapshot carrying typed proto data, the row_key, and school_id.
pub struct SnapshotRow {
    pub row_key: String,
    pub school_id: Option<Id>,
    pub insert_data: InsertData,
}

/// Load all rows from the given table as snapshot entries.
/// Returns an empty vec for unknown table numbers.
pub fn snapshot_table(conn: &mut Conn, table: i32) -> Result<Vec<SnapshotRow>> {
    snapshot_table_inner(conn, table, None)
}

/// Load rows from the given table that were created or updated at or after
/// `since` (unix timestamp in seconds). Used for incremental sync — only
/// rows that changed since the given timestamp are returned.
pub fn snapshot_table_since(conn: &mut Conn, table: i32, since: i64) -> Result<Vec<SnapshotRow>> {
    snapshot_table_inner(conn, table, Some(since))
}

fn snapshot_table_inner(
    conn: &mut Conn,
    table: i32,
    since: Option<i64>,
) -> Result<Vec<SnapshotRow>> {
    match table {
        TBL_USERS => query_users(conn, since),
        TBL_SCHOOLS => query_schools(conn, since),
        TBL_OWNERS => query_owners(conn, since),
        TBL_STUDENTS => query_students(conn, since),
        TBL_GUARDIANS => query_guardians(conn, since),
        TBL_DEPARTMENTS => query_departments(conn, since),
        TBL_TEACHERS => query_teachers(conn, since),
        TBL_STAFF => query_staff(conn, since),
        TBL_TERMS => query_terms(conn, since),
        TBL_CLASS_TEACHERS => query_class_teachers(conn, since),
        TBL_ENROLLMENTS => query_enrollments(conn, since),
        TBL_SUBJECTS => query_subject_teachers(conn, since),
        TBL_ATTENDANCE => query_attendance(conn, since),
        TBL_TIMETABLE => query_timetable(conn, since),
        TBL_LESSONS => query_lessons(conn, since),
        TBL_EXAMS => query_exams(conn, since),
        TBL_PAPERS => query_papers(conn, since),
        TBL_GRADES => query_grades(conn, since),
        TBL_FEES => query_fees(conn, since),
        TBL_INVOICES => query_invoices(conn, since),
        TBL_PAYMENTS => query_payments(conn, since),
        TBL_ANNOUNCEMENTS => query_announcements(conn, since),
        TBL_MASTERY => query_mastery(conn, since),
        TBL_AIUSAGE => query_aiusage(conn, since),
        TBL_SUBJECT_CATALOG => query_subject_catalog(conn, since),
        TBL_TOPICS => query_topics(conn, since),
        TBL_STREAMS => query_streams(conn, since),
        TBL_MPESA => query_mpesa(conn, since),
        TBL_ROLES => query_roles(conn, since),
        TBL_SCOPES => query_scopes(conn, since),
        TBL_PLANS => query_plans(conn, since),
        TBL_SUBSCRIPTIONS => query_subscriptions(conn, since),
        TBL_DISCOUNTS => query_discounts(conn, since),
        TBL_SCHEME_PAGES => query_scheme_pages(conn, since),
        TBL_ANSWER_PAGES => query_answer_pages(conn, since),
        _ => Ok(vec![]),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_school_id(s: Option<&str>) -> Option<Id> {
    s.and_then(|v| if v.is_empty() { None } else { v.parse().ok() })
}

/// Build the full SQL string, appending a WHERE clause for incremental sync
/// when `since` is `Some`. Tables with an `updated` column use
/// `WHERE updated >= $1 OR created >= $1`; tables without use
/// `WHERE created >= $1`.
fn build_sql(base: &str, since: Option<i64>, has_updated: bool) -> Option<String> {
    since.map(|_| {
        if has_updated {
            format!("{base} WHERE updated >= $1 OR created >= $1")
        } else {
            format!("{base} WHERE created >= $1")
        }
    })
}

/// Generic loader: runs the SQL query (with optional bind param) and maps
/// each row through the provided closure.
fn load_rows<T, F>(
    conn: &mut Conn,
    base: &str,
    has_updated: bool,
    since: Option<i64>,
    table_name: &str,
    map: F,
) -> Result<Vec<SnapshotRow>>
where
    T: diesel::deserialize::QueryableByName<diesel::sqlite::Sqlite> + 'static,
    F: Fn(&T) -> SnapshotRow,
{
    let rows: Vec<T> = match build_sql(base, since, has_updated) {
        None => diesel::sql_query(base).load(conn).map_err(|e| {
            error!("snapshot query failed for {table_name}: {e}");
            Error::Internal("internal server error".into())
        })?,
        Some(sql) => diesel::sql_query(&sql)
            .bind::<BigInt, _>(since.unwrap())
            .load(conn)
            .map_err(|e| {
                error!("snapshot_since query failed for {table_name}: {e}");
                Error::Internal("internal server error".into())
            })?,
    };
    Ok(rows.iter().map(map).collect())
}

// ---------------------------------------------------------------------------
// Per-table query functions
// ---------------------------------------------------------------------------

const SQL_USERS: &str = "SELECT id, phone, email, name, level, status, created, updated FROM users";

fn query_users(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<UserRow, _>(conn, SQL_USERS, true, since, "users", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::User(r.into())),
        },
    })
}

const SQL_SCHOOLS: &str = "SELECT id, name, motto, phone, email, county, domain, established, status, created, updated FROM schools";

fn query_schools(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<SchoolRow, _>(conn, SQL_SCHOOLS, true, since, "schools", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::School(r.into())),
        },
    })
}

const SQL_OWNERS: &str = "SELECT school, user, created FROM owners";

fn query_owners(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<OwnerRow, _>(conn, SQL_OWNERS, false, since, "owners", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::Owner(r.into())),
        },
    })
}

const SQL_STUDENTS: &str = "SELECT school, adm, user, name, dob, gender, documents, admitted, status, created, updated FROM students";

fn query_students(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<StudentRow, _>(conn, SQL_STUDENTS, true, since, "students", |r| {
        SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::Student(r.into())),
            },
        }
    })
}

const SQL_GUARDIANS: &str =
    "SELECT school, user, student, relationship, role, created, updated FROM guardians";

fn query_guardians(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<GuardianRow, _>(conn, SQL_GUARDIANS, true, since, "guardians", |r| {
        SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::Guardian(r.into())),
            },
        }
    })
}

const SQL_DEPARTMENTS: &str = "SELECT school, name, description, created, updated FROM departments";

fn query_departments(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<DepartmentRow, _>(conn, SQL_DEPARTMENTS, true, since, "departments", |r| {
        SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::Department(r.into())),
            },
        }
    })
}

const SQL_TEACHERS: &str =
    "SELECT school, user, hired, role, department, status, created, updated FROM teachers";

fn query_teachers(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<TeacherRow, _>(conn, SQL_TEACHERS, true, since, "teachers", |r| {
        SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::Teacher(r.into())),
            },
        }
    })
}

const SQL_STAFF: &str =
    "SELECT school, user, idnumber, role, department, status, created, updated FROM staff";

fn query_staff(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<StaffRow, _>(conn, SQL_STAFF, true, since, "staff", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::StaffMember(r.into())),
        },
    })
}

const SQL_TERMS: &str = "SELECT school, year, term, start, \"end\", created, updated FROM terms";

fn query_terms(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<TermRow, _>(conn, SQL_TERMS, true, since, "terms", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::Term(r.into())),
        },
    })
}

const SQL_CLASS_TEACHERS: &str = "SELECT school, year, term, grade, stream, teacher, start, \"end\", created FROM class_teachers";

fn query_class_teachers(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<ClassTeacherRow, _>(
        conn,
        SQL_CLASS_TEACHERS,
        false,
        since,
        "class_teachers",
        |r| SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::ClassTeacher(r.into())),
            },
        },
    )
}

const SQL_ENROLLMENTS: &str =
    "SELECT school, year, term, grade, stream, student, created FROM enrollments";

fn query_enrollments(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<EnrollmentRow, _>(conn, SQL_ENROLLMENTS, false, since, "enrollments", |r| {
        SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::Enrollment(r.into())),
            },
        }
    })
}

const SQL_SUBJECT_TEACHERS: &str =
    "SELECT school, year, term, grade, stream, subject, teacher, created FROM subject_teachers";

fn query_subject_teachers(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<SubjectTeacherRow, _>(
        conn,
        SQL_SUBJECT_TEACHERS,
        false,
        since,
        "subject_teachers",
        |r| SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::SubjectTeacher(r.into())),
            },
        },
    )
}

const SQL_ATTENDANCE: &str = "SELECT school, year, term, grade, stream, student, date, status, created, updated FROM attendance";

fn query_attendance(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<AttendanceRow, _>(conn, SQL_ATTENDANCE, true, since, "attendance", |r| {
        SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::Attendance(r.into())),
            },
        }
    })
}

const SQL_TIMETABLE: &str = "SELECT school, year, term, grade, stream, subject, teacher, day, start, \"end\", created, updated FROM timetable";

fn query_timetable(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<TimetableRow, _>(conn, SQL_TIMETABLE, true, since, "timetable", |r| {
        SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::Timetable(r.into())),
            },
        }
    })
}

const SQL_LESSONS: &str = "SELECT school, year, term, grade, stream, date, subject, teacher, created, updated FROM lessons";

fn query_lessons(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<LessonRow, _>(conn, SQL_LESSONS, true, since, "lessons", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::Lesson(r.into())),
        },
    })
}

const SQL_EXAMS: &str = "SELECT id, school, name, year, term, personalized, \"type\", start, \"end\", teacher, created, updated FROM exams";

fn query_exams(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<ExamRow, _>(conn, SQL_EXAMS, true, since, "exams", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::Exam(r.into())),
        },
    })
}

const SQL_PAPERS: &str = "SELECT school, exam, subject, paper, topic, \
    invigilator, start, \"end\", status, grade, stream, created, updated, \
    time_allowed_minutes, instructions FROM papers";

fn query_papers(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<PaperRow, _>(conn, SQL_PAPERS, true, since, "papers", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::Paper(r.into())),
        },
    })
}

const SQL_GRADES: &str =
    "SELECT school, exam, student, subject, paper, score, total, created, updated FROM grades";

fn query_grades(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<GradeRow, _>(conn, SQL_GRADES, true, since, "grades", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::Grade(r.into())),
        },
    })
}

const SQL_FEES: &str = "SELECT id, school, year, term, grade, title, description, amount, mandatory, due, created, updated FROM fees";

fn query_fees(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<FeeRow, _>(conn, SQL_FEES, true, since, "fees", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::Fee(r.into())),
        },
    })
}

const SQL_INVOICES: &str = "SELECT id, school, year, term, fee, description, student, amount, status, due, created, updated FROM invoices";

fn query_invoices(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<InvoiceRow, _>(conn, SQL_INVOICES, true, since, "invoices", |r| {
        SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::Invoice(r.into())),
            },
        }
    })
}

const SQL_PAYMENTS: &str = "SELECT id, invoice, school, student, amount, method, reference, recorder, date, created, updated FROM payments";

fn query_payments(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<PaymentRow, _>(conn, SQL_PAYMENTS, true, since, "payments", |r| {
        SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::Payment(r.into())),
            },
        }
    })
}

const SQL_ANNOUNCEMENTS: &str = "SELECT id, school, title, content, grade, stream, audience, author, created, updated FROM announcements";

fn query_announcements(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<AnnouncementRow, _>(conn, SQL_ANNOUNCEMENTS, true, since, "announcements", |r| {
        SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::Announcement(r.into())),
            },
        }
    })
}

const SQL_MASTERY: &str =
    "SELECT school, student, subject, topic, score, created, updated FROM mastery";

fn query_mastery(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<MasteryRow, _>(conn, SQL_MASTERY, true, since, "mastery", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::Mastery(r.into())),
        },
    })
}

const SQL_AIUSAGE: &str =
    "SELECT school, student, year, term, allocated, used, created, updated FROM aiusage";

fn query_aiusage(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<AiUsageRow, _>(conn, SQL_AIUSAGE, true, since, "aiusage", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::AiUsage(r.into())),
        },
    })
}

const SQL_ROLES: &str =
    "SELECT id, school, name, description, permissions, created, updated FROM roles";

fn query_roles(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<RoleRow, _>(conn, SQL_ROLES, true, since, "roles", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::Role(r.into())),
        },
    })
}

const SQL_SCOPES: &str = "SELECT school, user, role, created FROM scopes";

fn query_scopes(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<ScopeRow, _>(conn, SQL_SCOPES, false, since, "scopes", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::Scope(r.into())),
        },
    })
}

const SQL_PLANS: &str =
    "SELECT id, name, description, amount, levels, status, features, created, updated FROM plans";

fn query_plans(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<PlanRow, _>(conn, SQL_PLANS, true, since, "plans", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::Plan(r.into())),
        },
    })
}

const SQL_SUBSCRIPTIONS: &str = "SELECT school, plan, year, term, student, invoice, discount, status, created, updated FROM subscriptions";

fn query_subscriptions(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<SubscriptionRow, _>(conn, SQL_SUBSCRIPTIONS, true, since, "subscriptions", |r| {
        SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::Subscription(r.into())),
            },
        }
    })
}

const SQL_DISCOUNTS: &str =
    "SELECT school, plan, year, term, grade, amount, unit, created, updated FROM discounts";

fn query_discounts(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<DiscountRow, _>(conn, SQL_DISCOUNTS, true, since, "discounts", |r| {
        SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::Discount(r.into())),
            },
        }
    })
}

const SQL_SUBJECT_CATALOG: &str = "SELECT id, name, curriculum, created, updated FROM subjects";

fn query_subject_catalog(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<SubjectCatalogRow, _>(conn, SQL_SUBJECT_CATALOG, true, since, "subjects", |r| {
        SnapshotRow {
            row_key: r.row_key(),
            school_id: None,
            insert_data: InsertData {
                row: Some(insert_data::Row::SubjectCatalog(r.into())),
            },
        }
    })
}

const SQL_TOPICS: &str = "SELECT id, subject, grade, name, created, updated FROM topics";

fn query_topics(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<TopicRow, _>(conn, SQL_TOPICS, true, since, "topics", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: None,
        insert_data: InsertData {
            row: Some(insert_data::Row::Topic(r.into())),
        },
    })
}

const SQL_STREAMS: &str = "SELECT school, grade, stream, name, created, updated FROM streams";

fn query_streams(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<StreamRow, _>(conn, SQL_STREAMS, true, since, "streams", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::Stream(r.into())),
        },
    })
}

const SQL_MPESA: &str = "SELECT school, consumer_key, consumer_secret, passkey, shortcode, env, created, updated FROM mpesa";

fn query_mpesa(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<MpesaRow, _>(conn, SQL_MPESA, true, since, "mpesa", |r| SnapshotRow {
        row_key: r.row_key(),
        school_id: parse_school_id(r.school_id()),
        insert_data: InsertData {
            row: Some(insert_data::Row::Mpesa(r.into())),
        },
    })
}

const SQL_SCHEME_PAGES: &str =
    "SELECT school, exam, subject, paper, page, key, created FROM scheme_pages";

fn query_scheme_pages(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<SchemePageRow, _>(
        conn,
        SQL_SCHEME_PAGES,
        false, // no `updated` column
        since,
        "scheme_pages",
        |r| SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::SchemePage(r.into())),
            },
        },
    )
}

const SQL_ANSWER_PAGES: &str =
    "SELECT school, exam, student, subject, paper, page, key, created FROM answer_pages";

fn query_answer_pages(conn: &mut Conn, since: Option<i64>) -> Result<Vec<SnapshotRow>> {
    load_rows::<AnswerPageRow, _>(
        conn,
        SQL_ANSWER_PAGES,
        false, // no `updated` column
        since,
        "answer_pages",
        |r| SnapshotRow {
            row_key: r.row_key(),
            school_id: parse_school_id(r.school_id()),
            insert_data: InsertData {
                row: Some(insert_data::Row::AnswerPage(r.into())),
            },
        },
    )
}
