use crate::proto::services::sync::{InsertData, UpdateData, insert_data, update_data};
use crate::types::error::{Error, Result};
use diesel::SqliteConnection as Conn;

use super::{delete, insert, update};

pub const OP_INSERT: i32 = 0;
pub const OP_UPDATE: i32 = 1;
pub const OP_DELETE: i32 = 2;

pub fn apply_insert(conn: &mut Conn, table: i32, _row_key: &str, data: &InsertData) -> Result<()> {
    match &data.row {
        Some(insert_data::Row::User(r)) => insert::insert_user(conn, r),
        Some(insert_data::Row::School(r)) => insert::insert_school(conn, r),
        Some(insert_data::Row::Owner(r)) => insert::insert_owner(conn, r),
        Some(insert_data::Row::Student(r)) => insert::insert_student(conn, r),
        Some(insert_data::Row::Guardian(r)) => insert::insert_guardian(conn, r),
        Some(insert_data::Row::Department(r)) => insert::insert_department(conn, r),
        Some(insert_data::Row::Teacher(r)) => insert::insert_teacher(conn, r),
        Some(insert_data::Row::StaffMember(r)) => insert::insert_staff(conn, r),
        Some(insert_data::Row::Term(r)) => insert::insert_term(conn, r),
        Some(insert_data::Row::ClassTeacher(r)) => insert::insert_class_teacher(conn, r),
        Some(insert_data::Row::Enrollment(r)) => insert::insert_enrollment(conn, r),
        Some(insert_data::Row::Subject(r)) => insert::insert_subject(conn, r),
        Some(insert_data::Row::Attendance(r)) => insert::insert_attendance(conn, r),
        Some(insert_data::Row::Timetable(r)) => insert::insert_timetable(conn, r),
        Some(insert_data::Row::Lesson(r)) => insert::insert_lesson(conn, r),
        Some(insert_data::Row::Exam(r)) => insert::insert_exam(conn, r),
        Some(insert_data::Row::Paper(r)) => insert::insert_paper(conn, r),
        Some(insert_data::Row::Grade(r)) => insert::insert_grade(conn, r),
        Some(insert_data::Row::Fee(r)) => insert::insert_fee(conn, r),
        Some(insert_data::Row::Invoice(r)) => insert::insert_invoice(conn, r),
        Some(insert_data::Row::Payment(r)) => insert::insert_payment(conn, r),
        Some(insert_data::Row::Announcement(r)) => insert::insert_announcement(conn, r),
        Some(insert_data::Row::Mastery(r)) => insert::insert_mastery(conn, r),
        Some(insert_data::Row::AiUsage(r)) => insert::insert_ai_usage(conn, r),
        Some(insert_data::Row::Settings(r)) => insert::insert_settings(conn, r),
        Some(insert_data::Row::Role(r)) => insert::insert_role(conn, r),
        Some(insert_data::Row::Scope(r)) => insert::insert_scope(conn, r),
        Some(insert_data::Row::Plan(r)) => insert::insert_plan(conn, r),
        Some(insert_data::Row::Subscription(r)) => insert::insert_subscription(conn, r),
        Some(insert_data::Row::Discount(r)) => insert::insert_discount(conn, r),
        None => {
            tracing::error!("apply_insert: missing row data for table {table}");
            Err(Error::Internal)
        }
    }
}

pub fn apply_update(conn: &mut Conn, _table: i32, row_key: &str, data: &UpdateData) -> Result<()> {
    match &data.row {
        Some(update_data::Row::User(r)) => update::update_user(conn, row_key, r),
        Some(update_data::Row::School(r)) => update::update_school(conn, row_key, r),
        Some(update_data::Row::Student(r)) => update::update_student(conn, row_key, r),
        Some(update_data::Row::Guardian(r)) => update::update_guardian(conn, row_key, r),
        Some(update_data::Row::Department(r)) => update::update_department(conn, row_key, r),
        Some(update_data::Row::Teacher(r)) => update::update_teacher(conn, row_key, r),
        Some(update_data::Row::StaffMember(r)) => update::update_staff(conn, row_key, r),
        Some(update_data::Row::Term(r)) => update::update_term(conn, row_key, r),
        Some(update_data::Row::ClassTeacher(r)) => update::update_class_teacher(conn, row_key, r),
        Some(update_data::Row::Attendance(r)) => update::update_attendance(conn, row_key, r),
        Some(update_data::Row::Timetable(r)) => update::update_timetable(conn, row_key, r),
        Some(update_data::Row::Exam(r)) => update::update_exam(conn, row_key, r),
        Some(update_data::Row::Paper(r)) => update::update_paper(conn, row_key, r),
        Some(update_data::Row::Grade(r)) => update::update_grade(conn, row_key, r),
        Some(update_data::Row::Fee(r)) => update::update_fee(conn, row_key, r),
        Some(update_data::Row::Invoice(r)) => update::update_invoice(conn, row_key, r),
        Some(update_data::Row::Payment(r)) => update::update_payment(conn, row_key, r),
        Some(update_data::Row::Announcement(r)) => update::update_announcement(conn, row_key, r),
        Some(update_data::Row::Mastery(r)) => update::update_mastery(conn, row_key, r),
        Some(update_data::Row::AiUsage(r)) => update::update_ai_usage(conn, row_key, r),
        Some(update_data::Row::Settings(r)) => update::update_settings(conn, row_key, r),
        Some(update_data::Row::Role(r)) => update::update_role(conn, row_key, r),
        Some(update_data::Row::Plan(r)) => update::update_plan(conn, row_key, r),
        Some(update_data::Row::Subscription(r)) => update::update_subscription(conn, row_key, r),
        Some(update_data::Row::Discount(r)) => update::update_discount(conn, row_key, r),
        None => {
            tracing::error!("apply_update: missing update data for table {_table}");
            Err(Error::Internal)
        }
    }
}

pub fn apply_delete(conn: &mut Conn, table: i32, row_key: &str) -> Result<()> {
    match table {
        1 => delete::delete_user(conn, row_key),
        2 => delete::delete_school(conn, row_key),
        3 => delete::delete_owner(conn, row_key),
        4 => delete::delete_student(conn, row_key),
        5 => delete::delete_guardian(conn, row_key),
        6 => delete::delete_department(conn, row_key),
        7 => delete::delete_teacher(conn, row_key),
        8 => delete::delete_staff(conn, row_key),
        9 => delete::delete_term(conn, row_key),
        10 => delete::delete_class_teacher(conn, row_key),
        11 => delete::delete_enrollment(conn, row_key),
        12 => delete::delete_subject(conn, row_key),
        13 => delete::delete_attendance(conn, row_key),
        14 => delete::delete_timetable(conn, row_key),
        15 => delete::delete_lesson(conn, row_key),
        16 => delete::delete_exam(conn, row_key),
        17 => delete::delete_paper(conn, row_key),
        18 => delete::delete_grade(conn, row_key),
        19 => delete::delete_fee(conn, row_key),
        20 => delete::delete_invoice(conn, row_key),
        21 => delete::delete_payment(conn, row_key),
        22 => delete::delete_announcement(conn, row_key),
        23 => delete::delete_mastery(conn, row_key),
        24 => delete::delete_aiusage(conn, row_key),
        25 => delete::delete_settings(conn, row_key),
        26 => delete::delete_role(conn, row_key),
        27 => delete::delete_scope(conn, row_key),
        28 => delete::delete_plan(conn, row_key),
        29 => delete::delete_subscription(conn, row_key),
        30 => delete::delete_discount(conn, row_key),
        _ => {
            tracing::error!("apply_delete: unknown table {table}");
            Err(Error::Internal)
        }
    }
}

/// Top-level dispatcher. Replaces the old JSON-based `apply_mutation`.
pub fn apply_mutation(
    conn: &mut Conn,
    table: i32,
    op: i32,
    row_key: &str,
    insert_data: Option<&InsertData>,
    update_data: Option<&UpdateData>,
) -> Result<()> {
    match op {
        OP_INSERT => {
            let data = insert_data.ok_or_else(|| {
                tracing::error!("apply_mutation: missing insert data for op=0, table={table}");
                Error::Internal
            })?;
            apply_insert(conn, table, row_key, data)
        }
        OP_UPDATE => {
            let data = update_data.ok_or_else(|| {
                tracing::error!("apply_mutation: missing update data for op=1, table={table}");
                Error::Internal
            })?;
            apply_update(conn, table, row_key, data)
        }
        OP_DELETE => apply_delete(conn, table, row_key),
        _ => {
            tracing::error!("apply_mutation: unknown op {op} for table {table}");
            Err(Error::Internal)
        }
    }
}

/// Validates that the oneof variant in `InsertData` matches the declared table number.
pub fn validate_insert(table: i32, data: &InsertData) -> bool {
    matches!(
        (table, &data.row),
        (1, Some(insert_data::Row::User(_)))
            | (2, Some(insert_data::Row::School(_)))
            | (3, Some(insert_data::Row::Owner(_)))
            | (4, Some(insert_data::Row::Student(_)))
            | (5, Some(insert_data::Row::Guardian(_)))
            | (6, Some(insert_data::Row::Department(_)))
            | (7, Some(insert_data::Row::Teacher(_)))
            | (8, Some(insert_data::Row::StaffMember(_)))
            | (9, Some(insert_data::Row::Term(_)))
            | (10, Some(insert_data::Row::ClassTeacher(_)))
            | (11, Some(insert_data::Row::Enrollment(_)))
            | (12, Some(insert_data::Row::Subject(_)))
            | (13, Some(insert_data::Row::Attendance(_)))
            | (14, Some(insert_data::Row::Timetable(_)))
            | (15, Some(insert_data::Row::Lesson(_)))
            | (16, Some(insert_data::Row::Exam(_)))
            | (17, Some(insert_data::Row::Paper(_)))
            | (18, Some(insert_data::Row::Grade(_)))
            | (19, Some(insert_data::Row::Fee(_)))
            | (20, Some(insert_data::Row::Invoice(_)))
            | (21, Some(insert_data::Row::Payment(_)))
            | (22, Some(insert_data::Row::Announcement(_)))
            | (23, Some(insert_data::Row::Mastery(_)))
            | (24, Some(insert_data::Row::AiUsage(_)))
            | (25, Some(insert_data::Row::Settings(_)))
            | (26, Some(insert_data::Row::Role(_)))
            | (27, Some(insert_data::Row::Scope(_)))
            | (28, Some(insert_data::Row::Plan(_)))
            | (29, Some(insert_data::Row::Subscription(_)))
            | (30, Some(insert_data::Row::Discount(_)))
    )
}

/// Validates that the oneof variant in `UpdateData` matches the declared table number.
pub fn validate_update(table: i32, data: &UpdateData) -> bool {
    matches!(
        (table, &data.row),
        (1, Some(update_data::Row::User(_)))
            | (2, Some(update_data::Row::School(_)))
            | (4, Some(update_data::Row::Student(_)))
            | (5, Some(update_data::Row::Guardian(_)))
            | (6, Some(update_data::Row::Department(_)))
            | (7, Some(update_data::Row::Teacher(_)))
            | (8, Some(update_data::Row::StaffMember(_)))
            | (9, Some(update_data::Row::Term(_)))
            | (10, Some(update_data::Row::ClassTeacher(_)))
            | (13, Some(update_data::Row::Attendance(_)))
            | (14, Some(update_data::Row::Timetable(_)))
            | (16, Some(update_data::Row::Exam(_)))
            | (17, Some(update_data::Row::Paper(_)))
            | (18, Some(update_data::Row::Grade(_)))
            | (19, Some(update_data::Row::Fee(_)))
            | (20, Some(update_data::Row::Invoice(_)))
            | (21, Some(update_data::Row::Payment(_)))
            | (22, Some(update_data::Row::Announcement(_)))
            | (23, Some(update_data::Row::Mastery(_)))
            | (24, Some(update_data::Row::AiUsage(_)))
            | (25, Some(update_data::Row::Settings(_)))
            | (26, Some(update_data::Row::Role(_)))
            | (28, Some(update_data::Row::Plan(_)))
            | (29, Some(update_data::Row::Subscription(_)))
            | (30, Some(update_data::Row::Discount(_)))
    )
}
