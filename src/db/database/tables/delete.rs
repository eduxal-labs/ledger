#![allow(dead_code)]

use crate::types::error::{Error, Result};
use diesel::RunQueryDsl;
use diesel::SqliteConnection as Conn;
use diesel::sql_query;
use diesel::sql_types::{Integer, Nullable, SmallInt, Text};

#[derive(diesel::QueryableByName)]
struct DeletedPage {
    #[diesel(sql_type = SmallInt)]
    page: i16,
}

fn pk_parts(row_key: &str) -> Vec<&str> {
    row_key.split('|').collect()
}

pub fn delete_user(conn: &mut Conn, row_key: &str) -> Result<()> {
    sql_query("DELETE FROM users WHERE id = ?")
        .bind::<Text, _>(row_key)
        .execute(conn)?;
    Ok(())
}

pub fn delete_school(conn: &mut Conn, row_key: &str) -> Result<()> {
    sql_query("DELETE FROM schools WHERE id = ?")
        .bind::<Text, _>(row_key)
        .execute(conn)?;
    Ok(())
}

pub fn delete_owner(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 2 {
        return Err(Error::Internal("invalid row key for owner: expected at least 2 key parts".into()));
    }
    sql_query("DELETE FROM owners WHERE school = ? AND user = ?")
        .bind::<Text, _>(pk[0])
        .bind::<Text, _>(pk[1])
        .execute(conn)?;
    Ok(())
}

pub fn delete_student(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 2 {
        return Err(Error::Internal("invalid row key for student: expected at least 2 key parts".into()));
    }
    let adm: i32 = pk[1].parse().map_err(|e| Error::internal(e))?;
    sql_query("DELETE FROM students WHERE school = ? AND adm = ?")
        .bind::<Text, _>(pk[0])
        .bind::<Integer, _>(adm)
        .execute(conn)?;
    Ok(())
}

pub fn delete_guardian(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 3 {
        return Err(Error::Internal("invalid row key for guardian: expected at least 3 key parts".into()));
    }
    let student: i32 = pk[2].parse().map_err(|e| Error::internal(e))?;
    sql_query("DELETE FROM guardians WHERE school = ? AND user = ? AND student = ?")
        .bind::<Text, _>(pk[0])
        .bind::<Text, _>(pk[1])
        .bind::<Integer, _>(student)
        .execute(conn)?;
    Ok(())
}

pub fn delete_department(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 2 {
        return Err(Error::Internal("invalid row key for department: expected at least 2 key parts".into()));
    }
    sql_query("DELETE FROM departments WHERE school = ? AND name = ?")
        .bind::<Text, _>(pk[0])
        .bind::<Text, _>(pk[1])
        .execute(conn)?;
    Ok(())
}

pub fn delete_teacher(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 2 {
        return Err(Error::Internal("invalid row key for teacher: expected at least 2 key parts".into()));
    }
    sql_query("DELETE FROM teachers WHERE school = ? AND user = ?")
        .bind::<Text, _>(pk[0])
        .bind::<Text, _>(pk[1])
        .execute(conn)?;
    Ok(())
}

pub fn delete_staff(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 2 {
        return Err(Error::Internal("invalid row key for staff: expected at least 2 key parts".into()));
    }
    sql_query("DELETE FROM staff WHERE school = ? AND user = ?")
        .bind::<Text, _>(pk[0])
        .bind::<Text, _>(pk[1])
        .execute(conn)?;
    Ok(())
}

pub fn delete_term(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 3 {
        return Err(Error::Internal("invalid row key for term: expected at least 3 key parts".into()));
    }
    let year: i32 = pk[1].parse().map_err(|e| Error::internal(e))?;
    let term: i16 = pk[2].parse().map_err(|e| Error::internal(e))?;
    sql_query("DELETE FROM terms WHERE school = ? AND year = ? AND term = ?")
        .bind::<Text, _>(pk[0])
        .bind::<Integer, _>(year)
        .bind::<SmallInt, _>(term)
        .execute(conn)?;
    Ok(())
}

pub fn delete_class_teacher(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 6 {
        return Err(Error::Internal("invalid row key for class_teacher: expected at least 6 key parts".into()));
    }
    let year: i32 = pk[1].parse().map_err(|e| Error::internal(e))?;
    let term: i16 = pk[2].parse().map_err(|e| Error::internal(e))?;
    let grade: i16 = pk[3].parse().map_err(|e| Error::internal(e))?;
    let stream: i16 = pk[4].parse().map_err(|e| Error::internal(e))?;
    sql_query(
        "DELETE FROM class_teachers WHERE school = ? AND year = ? AND term = ? AND grade = ? AND stream = ? AND teacher = ?",
    )
    .bind::<Text, _>(pk[0])
    .bind::<Integer, _>(year)
    .bind::<SmallInt, _>(term)
    .bind::<SmallInt, _>(grade)
    .bind::<SmallInt, _>(stream)
    .bind::<Text, _>(pk[5])
    .execute(conn)?;
    Ok(())
}

pub fn delete_enrollment(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 6 {
        return Err(Error::Internal("invalid row key for enrollment: expected at least 6 key parts".into()));
    }
    let year: i32 = pk[1].parse().map_err(|e| Error::internal(e))?;
    let term: i16 = pk[2].parse().map_err(|e| Error::internal(e))?;
    let grade: i16 = pk[3].parse().map_err(|e| Error::internal(e))?;
    let stream: i16 = pk[4].parse().map_err(|e| Error::internal(e))?;
    let student: i32 = pk[5].parse().map_err(|e| Error::internal(e))?;
    sql_query(
        "DELETE FROM enrollments WHERE school = ? AND year = ? AND term = ? AND grade = ? AND stream = ? AND student = ?",
    )
    .bind::<Text, _>(pk[0])
    .bind::<Integer, _>(year)
    .bind::<SmallInt, _>(term)
    .bind::<SmallInt, _>(grade)
    .bind::<SmallInt, _>(stream)
    .bind::<Integer, _>(student)
    .execute(conn)?;
    Ok(())
}

pub fn delete_subject_teacher(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 6 {
        return Err(Error::Internal("invalid row key for subject_teacher: expected at least 6 key parts".into()));
    }
    let year: i32 = pk[1].parse().map_err(|e| Error::internal(e))?;
    let term: i16 = pk[2].parse().map_err(|e| Error::internal(e))?;
    let grade: i16 = pk[3].parse().map_err(|e| Error::internal(e))?;
    let stream: i16 = pk[4].parse().map_err(|e| Error::internal(e))?;
    let subject: i32 = pk[5].parse().map_err(|e| Error::internal(e))?;
    sql_query(
        "DELETE FROM subject_teachers WHERE school = ? AND year = ? AND term = ? AND grade = ? AND stream = ? AND subject = ?",
    )
    .bind::<Text, _>(pk[0])
    .bind::<Integer, _>(year)
    .bind::<SmallInt, _>(term)
    .bind::<SmallInt, _>(grade)
    .bind::<SmallInt, _>(stream)
    .bind::<Integer, _>(subject)
    .execute(conn)?;
    Ok(())
}

pub fn delete_attendance(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 7 {
        return Err(Error::Internal("invalid row key for attendance: expected at least 7 key parts".into()));
    }
    let year: i32 = pk[1].parse().map_err(|e| Error::internal(e))?;
    let term: i16 = pk[2].parse().map_err(|e| Error::internal(e))?;
    let grade: i16 = pk[3].parse().map_err(|e| Error::internal(e))?;
    let stream: i16 = pk[4].parse().map_err(|e| Error::internal(e))?;
    let student: i32 = pk[5].parse().map_err(|e| Error::internal(e))?;
    let date: i32 = pk[6].parse().map_err(|e| Error::internal(e))?;
    sql_query(
        "DELETE FROM attendance WHERE school = ? AND year = ? AND term = ? AND grade = ? \
         AND stream = ? AND student = ? AND date = ?",
    )
    .bind::<Text, _>(pk[0])
    .bind::<Integer, _>(year)
    .bind::<SmallInt, _>(term)
    .bind::<SmallInt, _>(grade)
    .bind::<SmallInt, _>(stream)
    .bind::<Integer, _>(student)
    .bind::<Integer, _>(date)
    .execute(conn)?;
    Ok(())
}

pub fn delete_timetable(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 8 {
        return Err(Error::Internal("invalid row key for timetable: expected at least 8 key parts".into()));
    }
    let year: i32 = pk[1].parse().map_err(|e| Error::internal(e))?;
    let term: i16 = pk[2].parse().map_err(|e| Error::internal(e))?;
    let grade: i16 = pk[3].parse().map_err(|e| Error::internal(e))?;
    let stream: i16 = pk[4].parse().map_err(|e| Error::internal(e))?;
    let subject: i16 = pk[5].parse().map_err(|e| Error::internal(e))?;
    let day: i16 = pk[6].parse().map_err(|e| Error::internal(e))?;
    let start: i32 = pk[7].parse().map_err(|e| Error::internal(e))?;
    sql_query(
        "DELETE FROM timetable WHERE school = ? AND year = ? AND term = ? AND grade = ? \
         AND stream = ? AND subject = ? AND day = ? AND start = ?",
    )
    .bind::<Text, _>(pk[0])
    .bind::<Integer, _>(year)
    .bind::<SmallInt, _>(term)
    .bind::<SmallInt, _>(grade)
    .bind::<SmallInt, _>(stream)
    .bind::<SmallInt, _>(subject)
    .bind::<SmallInt, _>(day)
    .bind::<Integer, _>(start)
    .execute(conn)?;
    Ok(())
}

pub fn delete_lesson(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 8 {
        return Err(Error::Internal("invalid row key for lesson: expected at least 8 key parts".into()));
    }
    let year: i32 = pk[1].parse().map_err(|e| Error::internal(e))?;
    let term: i16 = pk[2].parse().map_err(|e| Error::internal(e))?;
    let grade: i16 = pk[3].parse().map_err(|e| Error::internal(e))?;
    let stream: i16 = pk[4].parse().map_err(|e| Error::internal(e))?;
    let date: i32 = pk[5].parse().map_err(|e| Error::internal(e))?;
    let subject: i16 = pk[6].parse().map_err(|e| Error::internal(e))?;
    sql_query(
        "DELETE FROM lessons WHERE school = ? AND year = ? AND term = ? AND grade = ? \
         AND stream = ? AND date = ? AND subject = ? AND teacher = ?",
    )
    .bind::<Text, _>(pk[0])
    .bind::<Integer, _>(year)
    .bind::<SmallInt, _>(term)
    .bind::<SmallInt, _>(grade)
    .bind::<SmallInt, _>(stream)
    .bind::<Integer, _>(date)
    .bind::<SmallInt, _>(subject)
    .bind::<Text, _>(pk[7])
    .execute(conn)?;
    Ok(())
}

pub fn delete_exam(conn: &mut Conn, row_key: &str) -> Result<()> {
    sql_query("DELETE FROM exams WHERE id = ?")
        .bind::<Text, _>(row_key)
        .execute(conn)?;
    Ok(())
}

pub fn delete_paper(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 6 {
        return Err(Error::Internal("invalid row key for paper: expected at least 6 key parts".into()));
    }
    let subject: i32 = pk[2].parse().map_err(|e| Error::internal(e))?;
    let paper: Option<i16> = if pk[3].is_empty() {
        None
    } else {
        Some(pk[3].parse().map_err(|e| Error::internal(e))?)
    };
    let grade: i16 = pk[4].parse().map_err(|e| Error::internal(e))?;
    let stream: Option<i16> = if pk[5].is_empty() {
        None
    } else {
        Some(pk[5].parse().map_err(|e| Error::internal(e))?)
    };
    sql_query("DELETE FROM papers WHERE school = ? AND exam = ? AND subject = ? AND paper IS ? AND grade = ? AND stream IS ?")
        .bind::<Text, _>(pk[0])
        .bind::<Text, _>(pk[1])
        .bind::<Integer, _>(subject)
        .bind::<Nullable<SmallInt>, _>(paper)
        .bind::<SmallInt, _>(grade)
        .bind::<Nullable<SmallInt>, _>(stream)
        .execute(conn)?;
    Ok(())
}

pub fn delete_grade(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 5 {
        return Err(Error::Internal("invalid row key for grade: expected at least 5 key parts".into()));
    }
    let student: i32 = pk[2].parse().map_err(|e| Error::internal(e))?;
    let subject: i32 = pk[3].parse().map_err(|e| Error::internal(e))?;
    let paper: Option<i16> = if pk[4].is_empty() {
        None
    } else {
        Some(pk[4].parse().map_err(|e| Error::internal(e))?)
    };
    sql_query(
        "DELETE FROM grades WHERE school = ? AND exam = ? AND student = ? AND subject = ? AND paper IS ?",
    )
    .bind::<Text, _>(pk[0])
    .bind::<Text, _>(pk[1])
    .bind::<Integer, _>(student)
    .bind::<Integer, _>(subject)
    .bind::<Nullable<SmallInt>, _>(paper)
    .execute(conn)?;
    Ok(())
}

pub fn delete_fee(conn: &mut Conn, row_key: &str) -> Result<()> {
    sql_query("DELETE FROM fees WHERE id = ?")
        .bind::<Text, _>(row_key)
        .execute(conn)?;
    Ok(())
}

pub fn delete_invoice(conn: &mut Conn, row_key: &str) -> Result<()> {
    sql_query("DELETE FROM invoices WHERE id = ?")
        .bind::<Text, _>(row_key)
        .execute(conn)?;
    Ok(())
}

pub fn delete_payment(conn: &mut Conn, row_key: &str) -> Result<()> {
    sql_query("DELETE FROM payments WHERE id = ?")
        .bind::<Text, _>(row_key)
        .execute(conn)?;
    Ok(())
}

pub fn delete_announcement(conn: &mut Conn, row_key: &str) -> Result<()> {
    sql_query("DELETE FROM announcements WHERE id = ?")
        .bind::<Text, _>(row_key)
        .execute(conn)?;
    Ok(())
}

pub fn delete_mastery(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 4 {
        return Err(Error::Internal("invalid row key for mastery: expected at least 4 key parts".into()));
    }
    let student: i32 = pk[1].parse().map_err(|e| Error::internal(e))?;
    let subject: i32 = pk[2].parse().map_err(|e| Error::internal(e))?;
    let topic: i32 = pk[3].parse().map_err(|e| Error::internal(e))?;
    sql_query("DELETE FROM mastery WHERE school = ? AND student = ? AND subject = ? AND topic = ?")
        .bind::<Text, _>(pk[0])
        .bind::<Integer, _>(student)
        .bind::<Integer, _>(subject)
        .bind::<Integer, _>(topic)
        .execute(conn)?;
    Ok(())
}

pub fn delete_aiusage(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 4 {
        return Err(Error::Internal("invalid row key for aiusage: expected at least 4 key parts".into()));
    }
    let student: i32 = pk[1].parse().map_err(|e| Error::internal(e))?;
    let year: i32 = pk[2].parse().map_err(|e| Error::internal(e))?;
    let term: i16 = pk[3].parse().map_err(|e| Error::internal(e))?;
    sql_query("DELETE FROM aiusage WHERE school = ? AND student = ? AND year = ? AND term = ?")
        .bind::<Text, _>(pk[0])
        .bind::<Integer, _>(student)
        .bind::<Integer, _>(year)
        .bind::<SmallInt, _>(term)
        .execute(conn)?;
    Ok(())
}

pub fn delete_role(conn: &mut Conn, row_key: &str) -> Result<()> {
    sql_query("DELETE FROM roles WHERE id = ?")
        .bind::<Text, _>(row_key)
        .execute(conn)?;
    Ok(())
}

pub fn delete_scope(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 3 {
        return Err(Error::Internal("invalid row key for scope: expected at least 3 key parts".into()));
    }
    // school can be empty string for NULL (system-scoped roles)
    if pk[0].is_empty() {
        sql_query("DELETE FROM scopes WHERE school IS NULL AND user = ? AND role = ?")
            .bind::<Text, _>(pk[1])
            .bind::<Text, _>(pk[2])
            .execute(conn)?;
    } else {
        sql_query("DELETE FROM scopes WHERE school = ? AND user = ? AND role = ?")
            .bind::<Text, _>(pk[0])
            .bind::<Text, _>(pk[1])
            .bind::<Text, _>(pk[2])
            .execute(conn)?;
    }
    Ok(())
}

pub fn delete_plan(conn: &mut Conn, row_key: &str) -> Result<()> {
    sql_query("DELETE FROM plans WHERE id = ?")
        .bind::<Text, _>(row_key)
        .execute(conn)?;
    Ok(())
}

pub fn delete_subscription(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 5 {
        return Err(Error::Internal("invalid row key for subscription: expected at least 5 key parts".into()));
    }
    let year: i32 = pk[2].parse().map_err(|e| Error::internal(e))?;
    let term: i16 = pk[3].parse().map_err(|e| Error::internal(e))?;
    let student: i32 = pk[4].parse().map_err(|e| Error::internal(e))?;
    sql_query(
        "DELETE FROM subscriptions WHERE school = ? AND plan = ? AND year = ? AND term = ? AND student = ?",
    )
    .bind::<Text, _>(pk[0])
    .bind::<Text, _>(pk[1])
    .bind::<Integer, _>(year)
    .bind::<SmallInt, _>(term)
    .bind::<Integer, _>(student)
    .execute(conn)?;
    Ok(())
}

pub fn delete_discount(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 5 {
        return Err(Error::Internal("invalid row key for discount: expected at least 5 key parts".into()));
    }
    let year: i32 = pk[2].parse().map_err(|e| Error::internal(e))?;
    let term: i16 = pk[3].parse().map_err(|e| Error::internal(e))?;
    let grade: i16 = pk[4].parse().map_err(|e| Error::internal(e))?;
    sql_query(
        "DELETE FROM discounts WHERE school = ? AND plan = ? AND year = ? AND term = ? AND grade = ?",
    )
    .bind::<Text, _>(pk[0])
    .bind::<Text, _>(pk[1])
    .bind::<Integer, _>(year)
    .bind::<SmallInt, _>(term)
    .bind::<SmallInt, _>(grade)
    .execute(conn)?;
    Ok(())
}

pub fn delete_subject_catalog(conn: &mut Conn, id: i32) -> Result<()> {
    sql_query("DELETE FROM subjects WHERE id = ?")
        .bind::<Integer, _>(id)
        .execute(conn)?;
    Ok(())
}

pub fn delete_topic(conn: &mut Conn, id: i32) -> Result<()> {
    sql_query("DELETE FROM topics WHERE id = ?")
        .bind::<Integer, _>(id)
        .execute(conn)?;
    Ok(())
}

pub fn delete_stream(conn: &mut Conn, school: &str, grade: i16, stream: i16) -> Result<()> {
    sql_query("DELETE FROM streams WHERE school = ? AND grade = ? AND stream = ?")
        .bind::<Text, _>(school)
        .bind::<SmallInt, _>(grade)
        .bind::<SmallInt, _>(stream)
        .execute(conn)?;
    Ok(())
}

pub fn delete_mpesa(conn: &mut Conn, school: &str) -> Result<()> {
    sql_query("DELETE FROM mpesa WHERE school = ?")
        .bind::<Text, _>(school)
        .execute(conn)?;
    Ok(())
}

/// Delete all scheme pages for the given (school, exam, subject, paper) combination.
/// Returns the 0-indexed page numbers that were deleted, so the caller can
/// construct row_keys for the changelog.
pub fn delete_scheme_pages(
    conn: &mut Conn,
    school: &str,
    exam: &str,
    subject: i32,
    paper: Option<i16>,
) -> Result<Vec<i16>> {
    let existing: Vec<DeletedPage> = sql_query(
        "SELECT page FROM scheme_pages \
         WHERE school = ? AND exam = ? AND subject = ? AND paper IS ?",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(exam)
    .bind::<Integer, _>(subject)
    .bind::<Nullable<SmallInt>, _>(paper)
    .load(conn)?;

    sql_query(
        "DELETE FROM scheme_pages \
         WHERE school = ? AND exam = ? AND subject = ? AND paper IS ?",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(exam)
    .bind::<Integer, _>(subject)
    .bind::<Nullable<SmallInt>, _>(paper)
    .execute(conn)?;

    Ok(existing.into_iter().map(|r| r.page).collect())
}

/// Delete all answer pages for the given (school, exam, student, subject, paper) combination.
/// Returns the 0-indexed page numbers that were deleted, so the caller can
/// construct row_keys for the changelog.
pub fn delete_answer_pages(
    conn: &mut Conn,
    school: &str,
    exam: &str,
    student: i32,
    subject: i32,
    paper: Option<i16>,
) -> Result<Vec<i16>> {
    let existing: Vec<DeletedPage> = sql_query(
        "SELECT page FROM answer_pages \
         WHERE school = ? AND exam = ? AND student = ? AND subject = ? AND paper IS ?",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(exam)
    .bind::<Integer, _>(student)
    .bind::<Integer, _>(subject)
    .bind::<Nullable<SmallInt>, _>(paper)
    .load(conn)?;

    sql_query(
        "DELETE FROM answer_pages \
         WHERE school = ? AND exam = ? AND student = ? AND subject = ? AND paper IS ?",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(exam)
    .bind::<Integer, _>(student)
    .bind::<Integer, _>(subject)
    .bind::<Nullable<SmallInt>, _>(paper)
    .execute(conn)?;

    Ok(existing.into_iter().map(|r| r.page).collect())
}
