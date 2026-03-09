use crate::types::error::{Error, Result};
use diesel::RunQueryDsl;
use diesel::SqliteConnection as Conn;
use diesel::sql_query;
use diesel::sql_types::{Integer, Nullable, SmallInt, Text};

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
        return Err(Error::Internal);
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
        return Err(Error::Internal);
    }
    let adm: i32 = pk[1].parse().map_err(|_| Error::Internal)?;
    sql_query("DELETE FROM students WHERE school = ? AND adm = ?")
        .bind::<Text, _>(pk[0])
        .bind::<Integer, _>(adm)
        .execute(conn)?;
    Ok(())
}

pub fn delete_guardian(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 3 {
        return Err(Error::Internal);
    }
    let student: i32 = pk[2].parse().map_err(|_| Error::Internal)?;
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
        return Err(Error::Internal);
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
        return Err(Error::Internal);
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
        return Err(Error::Internal);
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
        return Err(Error::Internal);
    }
    let year: i32 = pk[1].parse().map_err(|_| Error::Internal)?;
    let term: i16 = pk[2].parse().map_err(|_| Error::Internal)?;
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
        return Err(Error::Internal);
    }
    let year: i32 = pk[1].parse().map_err(|_| Error::Internal)?;
    let term: i16 = pk[2].parse().map_err(|_| Error::Internal)?;
    let grade: i16 = pk[3].parse().map_err(|_| Error::Internal)?;
    let stream: i16 = pk[4].parse().map_err(|_| Error::Internal)?;
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
        return Err(Error::Internal);
    }
    let year: i32 = pk[1].parse().map_err(|_| Error::Internal)?;
    let term: i16 = pk[2].parse().map_err(|_| Error::Internal)?;
    let grade: i16 = pk[3].parse().map_err(|_| Error::Internal)?;
    let stream: i16 = pk[4].parse().map_err(|_| Error::Internal)?;
    let student: i32 = pk[5].parse().map_err(|_| Error::Internal)?;
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

pub fn delete_subject(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 6 {
        return Err(Error::Internal);
    }
    let year: i32 = pk[1].parse().map_err(|_| Error::Internal)?;
    let term: i16 = pk[2].parse().map_err(|_| Error::Internal)?;
    let grade: i16 = pk[3].parse().map_err(|_| Error::Internal)?;
    let stream: i16 = pk[4].parse().map_err(|_| Error::Internal)?;
    let subject: i16 = pk[5].parse().map_err(|_| Error::Internal)?;
    sql_query(
        "DELETE FROM subjects WHERE school = ? AND year = ? AND term = ? AND grade = ? AND stream = ? AND subject = ?",
    )
    .bind::<Text, _>(pk[0])
    .bind::<Integer, _>(year)
    .bind::<SmallInt, _>(term)
    .bind::<SmallInt, _>(grade)
    .bind::<SmallInt, _>(stream)
    .bind::<SmallInt, _>(subject)
    .execute(conn)?;
    Ok(())
}

pub fn delete_attendance(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 7 {
        return Err(Error::Internal);
    }
    let year: i32 = pk[1].parse().map_err(|_| Error::Internal)?;
    let term: i16 = pk[2].parse().map_err(|_| Error::Internal)?;
    let grade: i16 = pk[3].parse().map_err(|_| Error::Internal)?;
    let stream: i16 = pk[4].parse().map_err(|_| Error::Internal)?;
    let student: i32 = pk[5].parse().map_err(|_| Error::Internal)?;
    let date: i32 = pk[6].parse().map_err(|_| Error::Internal)?;
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
        return Err(Error::Internal);
    }
    let year: i32 = pk[1].parse().map_err(|_| Error::Internal)?;
    let term: i16 = pk[2].parse().map_err(|_| Error::Internal)?;
    let grade: i16 = pk[3].parse().map_err(|_| Error::Internal)?;
    let stream: i16 = pk[4].parse().map_err(|_| Error::Internal)?;
    let subject: i16 = pk[5].parse().map_err(|_| Error::Internal)?;
    let day: i16 = pk[6].parse().map_err(|_| Error::Internal)?;
    let start: i32 = pk[7].parse().map_err(|_| Error::Internal)?;
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
        return Err(Error::Internal);
    }
    let year: i32 = pk[1].parse().map_err(|_| Error::Internal)?;
    let term: i16 = pk[2].parse().map_err(|_| Error::Internal)?;
    let grade: i16 = pk[3].parse().map_err(|_| Error::Internal)?;
    let stream: i16 = pk[4].parse().map_err(|_| Error::Internal)?;
    let date: i32 = pk[5].parse().map_err(|_| Error::Internal)?;
    let subject: i16 = pk[6].parse().map_err(|_| Error::Internal)?;
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
    if pk.len() < 4 {
        return Err(Error::Internal);
    }
    let subject: i16 = pk[2].parse().map_err(|_| Error::Internal)?;
    let paper: Option<i16> = if pk[3].is_empty() {
        None
    } else {
        Some(pk[3].parse().map_err(|_| Error::Internal)?)
    };
    sql_query("DELETE FROM papers WHERE school = ? AND exam = ? AND subject = ? AND paper IS ?")
        .bind::<Text, _>(pk[0])
        .bind::<Text, _>(pk[1])
        .bind::<SmallInt, _>(subject)
        .bind::<Nullable<SmallInt>, _>(paper)
        .execute(conn)?;
    Ok(())
}

pub fn delete_grade(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 5 {
        return Err(Error::Internal);
    }
    let student: i32 = pk[2].parse().map_err(|_| Error::Internal)?;
    let subject: i16 = pk[3].parse().map_err(|_| Error::Internal)?;
    let paper: Option<i16> = if pk[4].is_empty() {
        None
    } else {
        Some(pk[4].parse().map_err(|_| Error::Internal)?)
    };
    sql_query(
        "DELETE FROM grades WHERE school = ? AND exam = ? AND student = ? AND subject = ? AND paper IS ?",
    )
    .bind::<Text, _>(pk[0])
    .bind::<Text, _>(pk[1])
    .bind::<Integer, _>(student)
    .bind::<SmallInt, _>(subject)
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
    if pk.len() < 5 {
        return Err(Error::Internal);
    }
    let student: i32 = pk[1].parse().map_err(|_| Error::Internal)?;
    let grade: i16 = pk[2].parse().map_err(|_| Error::Internal)?;
    let subject: i16 = pk[3].parse().map_err(|_| Error::Internal)?;
    let topic: i16 = pk[4].parse().map_err(|_| Error::Internal)?;
    sql_query(
        "DELETE FROM mastery WHERE school = ? AND student = ? AND grade = ? AND subject = ? AND topic = ?",
    )
    .bind::<Text, _>(pk[0])
    .bind::<Integer, _>(student)
    .bind::<SmallInt, _>(grade)
    .bind::<SmallInt, _>(subject)
    .bind::<SmallInt, _>(topic)
    .execute(conn)?;
    Ok(())
}

pub fn delete_aiusage(conn: &mut Conn, row_key: &str) -> Result<()> {
    let pk = pk_parts(row_key);
    if pk.len() < 4 {
        return Err(Error::Internal);
    }
    let student: i32 = pk[1].parse().map_err(|_| Error::Internal)?;
    let year: i32 = pk[2].parse().map_err(|_| Error::Internal)?;
    let term: i16 = pk[3].parse().map_err(|_| Error::Internal)?;
    sql_query("DELETE FROM aiusage WHERE school = ? AND student = ? AND year = ? AND term = ?")
        .bind::<Text, _>(pk[0])
        .bind::<Integer, _>(student)
        .bind::<Integer, _>(year)
        .bind::<SmallInt, _>(term)
        .execute(conn)?;
    Ok(())
}

pub fn delete_settings(conn: &mut Conn, row_key: &str) -> Result<()> {
    sql_query("DELETE FROM settings WHERE school = ?")
        .bind::<Text, _>(row_key)
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
        return Err(Error::Internal);
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
        return Err(Error::Internal);
    }
    let year: i32 = pk[2].parse().map_err(|_| Error::Internal)?;
    let term: i16 = pk[3].parse().map_err(|_| Error::Internal)?;
    let student: i32 = pk[4].parse().map_err(|_| Error::Internal)?;
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
        return Err(Error::Internal);
    }
    let year: i32 = pk[2].parse().map_err(|_| Error::Internal)?;
    let term: i16 = pk[3].parse().map_err(|_| Error::Internal)?;
    let grade: i16 = pk[4].parse().map_err(|_| Error::Internal)?;
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
