#![allow(dead_code)]

use crate::proto::services::sync::*;
use crate::types::error::Result;
use diesel::RunQueryDsl;
use diesel::SqliteConnection as Conn;
use diesel::sql_query;
use diesel::sql_types::{BigInt, Binary, Bool, Float, Integer, Nullable, SmallInt, Text};

// ---------------------------------------------------------------------------
// users (PK: id)
// ---------------------------------------------------------------------------

pub fn update_user(conn: &mut Conn, row_key: &str, row: &UpdateUserPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "UPDATE users SET \
         phone = COALESCE(?, phone), \
         email = COALESCE(?, email), \
         name = COALESCE(?, name), \
         level = COALESCE(?, level), \
         status = COALESCE(?, status), \
         updated = ? \
         WHERE id = ?",
    )
    .bind::<Nullable<Text>, _>(row.phone.as_deref())
    .bind::<Nullable<Text>, _>(row.email.as_deref())
    .bind::<Nullable<Text>, _>(row.name.as_deref())
    .bind::<Nullable<SmallInt>, _>(row.level.map(|v| v as i16))
    .bind::<Nullable<SmallInt>, _>(row.status.map(|v| v as i16))
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(row_key)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// schools (PK: id)
// ---------------------------------------------------------------------------

pub fn update_school(conn: &mut Conn, row_key: &str, row: &UpdateSchoolPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "UPDATE schools SET \
         name = COALESCE(?, name), \
         motto = COALESCE(?, motto), \
         phone = COALESCE(?, phone), \
         email = COALESCE(?, email), \
         county = COALESCE(?, county), \
         domain = COALESCE(?, domain), \
         established = COALESCE(?, established), \
         status = COALESCE(?, status), \
         updated = ? \
         WHERE id = ?",
    )
    .bind::<Nullable<Text>, _>(row.name.as_deref())
    .bind::<Nullable<Text>, _>(row.motto.as_deref())
    .bind::<Nullable<Text>, _>(row.phone.as_deref())
    .bind::<Nullable<Text>, _>(row.email.as_deref())
    .bind::<Nullable<Integer>, _>(row.county)
    .bind::<Nullable<Text>, _>(row.domain.as_deref())
    .bind::<Nullable<Integer>, _>(row.established)
    .bind::<Nullable<SmallInt>, _>(row.status.map(|v| v as i16))
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(row_key)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// students (PK: school|adm)
// ---------------------------------------------------------------------------

pub fn update_student(conn: &mut Conn, row_key: &str, row: &UpdateStudentPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let parts: Vec<&str> = row_key.split('|').collect();
    let (school, adm) = (
        parts[0],
        parts[1]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
    );
    sql_query(
        "UPDATE students SET \
         user = COALESCE(?, user), \
         name = COALESCE(?, name), \
         dob = COALESCE(?, dob), \
         gender = COALESCE(?, gender), \
         documents = COALESCE(?, documents), \
         admitted = COALESCE(?, admitted), \
         status = COALESCE(?, status), \
         updated = ? \
         WHERE school = ? AND adm = ?",
    )
    .bind::<Nullable<Text>, _>(row.user.as_deref())
    .bind::<Nullable<Text>, _>(row.name.as_deref())
    .bind::<Nullable<Integer>, _>(row.dob)
    .bind::<Nullable<SmallInt>, _>(row.gender.map(|v| v as i16))
    .bind::<Nullable<Text>, _>(row.documents.as_deref())
    .bind::<Nullable<Integer>, _>(row.admitted)
    .bind::<Nullable<SmallInt>, _>(row.status.map(|v| v as i16))
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(school)
    .bind::<Integer, _>(adm)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// guardians (PK: school|user|student)
// ---------------------------------------------------------------------------

pub fn update_guardian(conn: &mut Conn, row_key: &str, row: &UpdateGuardianPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let parts: Vec<&str> = row_key.split('|').collect();
    let (school, user, student) = (
        parts[0],
        parts[1],
        parts[2]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
    );
    sql_query(
        "UPDATE guardians SET \
         relationship = COALESCE(?, relationship), \
         role = COALESCE(?, role), \
         updated = ? \
         WHERE school = ? AND user = ? AND student = ?",
    )
    .bind::<Nullable<SmallInt>, _>(row.relationship.map(|v| v as i16))
    .bind::<Nullable<SmallInt>, _>(row.role.map(|v| v as i16))
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(school)
    .bind::<Text, _>(user)
    .bind::<Integer, _>(student)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// departments (PK: school|name)
// ---------------------------------------------------------------------------

pub fn update_department(
    conn: &mut Conn,
    row_key: &str,
    row: &UpdateDepartmentPayload,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let parts: Vec<&str> = row_key.split('|').collect();
    let (school, name) = (parts[0], parts[1]);
    sql_query(
        "UPDATE departments SET \
         description = COALESCE(?, description), \
         updated = ? \
         WHERE school = ? AND name = ?",
    )
    .bind::<Nullable<Text>, _>(row.description.as_deref())
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(school)
    .bind::<Text, _>(name)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// teachers (PK: school|user)
// ---------------------------------------------------------------------------

pub fn update_teacher(conn: &mut Conn, row_key: &str, row: &UpdateTeacherPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let parts: Vec<&str> = row_key.split('|').collect();
    let (school, user) = (parts[0], parts[1]);
    sql_query(
        "UPDATE teachers SET \
         hired = COALESCE(?, hired), \
         role = COALESCE(?, role), \
         department = COALESCE(?, department), \
         status = COALESCE(?, status), \
         updated = ? \
         WHERE school = ? AND user = ?",
    )
    .bind::<Nullable<Integer>, _>(row.hired)
    .bind::<Nullable<Text>, _>(row.role.as_deref())
    .bind::<Nullable<Text>, _>(row.department.as_deref())
    .bind::<Nullable<SmallInt>, _>(row.status.map(|v| v as i16))
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(school)
    .bind::<Text, _>(user)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// staff (PK: school|user)
// ---------------------------------------------------------------------------

pub fn update_staff(conn: &mut Conn, row_key: &str, row: &UpdateStaffPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let parts: Vec<&str> = row_key.split('|').collect();
    let (school, user) = (parts[0], parts[1]);
    sql_query(
        "UPDATE staff SET \
         idnumber = COALESCE(?, idnumber), \
         role = COALESCE(?, role), \
         department = COALESCE(?, department), \
         status = COALESCE(?, status), \
         updated = ? \
         WHERE school = ? AND user = ?",
    )
    .bind::<Nullable<Text>, _>(row.idnumber.as_deref())
    .bind::<Nullable<Text>, _>(row.role.as_deref())
    .bind::<Nullable<Text>, _>(row.department.as_deref())
    .bind::<Nullable<SmallInt>, _>(row.status.map(|v| v as i16))
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(school)
    .bind::<Text, _>(user)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// terms (PK: school|year|term)
// ---------------------------------------------------------------------------

pub fn update_term(conn: &mut Conn, row_key: &str, row: &UpdateTermPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let parts: Vec<&str> = row_key.split('|').collect();
    let (school, year, term) = (
        parts[0],
        parts[1]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[2]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
    );
    sql_query(
        "UPDATE terms SET \
         start = COALESCE(?, start), \
         \"end\" = COALESCE(?, \"end\"), \
         updated = ? \
         WHERE school = ? AND year = ? AND term = ?",
    )
    .bind::<Nullable<BigInt>, _>(row.start)
    .bind::<Nullable<BigInt>, _>(row.end)
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(school)
    .bind::<Integer, _>(year)
    .bind::<SmallInt, _>(term)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// class_teachers (PK: school|year|term|grade|stream|teacher)
// NOTE: no `updated` column on this table.
// In the new action-based system class teachers are assigned/unassigned,
// not updated. This helper is kept for potential future use.
// ---------------------------------------------------------------------------

pub fn update_class_teacher(
    conn: &mut Conn,
    row_key: &str,
    start: Option<i32>,
    end: Option<i32>,
) -> Result<()> {
    let parts: Vec<&str> = row_key.split('|').collect();
    let (school, year, term, grade, stream, teacher) = (
        parts[0],
        parts[1]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[2]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[3]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[4]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[5],
    );
    sql_query(
        "UPDATE class_teachers SET \
         start = COALESCE(?, start), \
         \"end\" = COALESCE(?, \"end\") \
         WHERE school = ? AND year = ? AND term = ? AND grade = ? AND stream = ? AND teacher = ?",
    )
    .bind::<Nullable<Integer>, _>(start)
    .bind::<Nullable<Integer>, _>(end)
    .bind::<Text, _>(school)
    .bind::<Integer, _>(year)
    .bind::<SmallInt, _>(term)
    .bind::<SmallInt, _>(grade)
    .bind::<SmallInt, _>(stream)
    .bind::<Text, _>(teacher)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// attendance (PK: school|year|term|grade|stream|student|date)
// ---------------------------------------------------------------------------

// In the new action-based system, attendance is marked via batch records
// in MarkAttendancePayload. This helper updates a single attendance row.
pub fn update_attendance(conn: &mut Conn, row_key: &str, status: i16) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let parts: Vec<&str> = row_key.split('|').collect();
    let (school, year, term, grade, stream, student, date) = (
        parts[0],
        parts[1]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[2]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[3]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[4]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[5]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[6]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
    );
    sql_query(
        "UPDATE attendance SET \
         status = ?, \
         updated = ? \
         WHERE school = ? AND year = ? AND term = ? AND grade = ? AND stream = ? AND student = ? AND date = ?",
    )
    .bind::<SmallInt, _>(status)
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(school)
    .bind::<Integer, _>(year)
    .bind::<SmallInt, _>(term)
    .bind::<SmallInt, _>(grade)
    .bind::<SmallInt, _>(stream)
    .bind::<Integer, _>(student)
    .bind::<Integer, _>(date)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// timetable (PK: school|year|term|grade|stream|subject|day|start)
// ---------------------------------------------------------------------------

pub fn update_timetable(
    conn: &mut Conn,
    row_key: &str,
    row: &UpdateTimetableEntryPayload,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let parts: Vec<&str> = row_key.split('|').collect();
    let (school, year, term, grade, stream, subject, day, start) = (
        parts[0],
        parts[1]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[2]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[3]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[4]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[5]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[6]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[7]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
    );
    sql_query(
        "UPDATE timetable SET \
         teacher = COALESCE(?, teacher), \
         \"end\" = COALESCE(?, \"end\"), \
         updated = ? \
         WHERE school = ? AND year = ? AND term = ? AND grade = ? AND stream = ? \
         AND subject = ? AND day = ? AND start = ?",
    )
    .bind::<Nullable<Text>, _>(row.teacher.as_deref())
    .bind::<Nullable<Integer>, _>(row.end)
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(school)
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

// ---------------------------------------------------------------------------
// exams (PK: id)
// ---------------------------------------------------------------------------

pub fn update_exam(conn: &mut Conn, row_key: &str, row: &UpdateExamPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "UPDATE exams SET \
         name = COALESCE(?, name), \
         personalized = COALESCE(?, personalized), \
         \"type\" = COALESCE(?, \"type\"), \
         start = COALESCE(?, start), \
         \"end\" = COALESCE(?, \"end\"), \
         teacher = COALESCE(?, teacher), \
         updated = ? \
         WHERE id = ?",
    )
    .bind::<Nullable<Text>, _>(row.name.as_deref())
    .bind::<Nullable<Bool>, _>(row.personalized)
    .bind::<Nullable<SmallInt>, _>(row.r#type.map(|v| v as i16))
    .bind::<Nullable<Integer>, _>(row.start)
    .bind::<Nullable<Integer>, _>(row.end)
    .bind::<Nullable<Text>, _>(row.teacher.as_deref())
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(row_key)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// papers (PK: school|exam|subject|paper|grade|stream)
// paper and stream are nullable in PK
// ---------------------------------------------------------------------------

pub fn update_paper(conn: &mut Conn, row_key: &str, row: &UpdatePaperPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let parts: Vec<&str> = row_key.split('|').collect();
    let school = parts[0];
    let exam = parts[1];
    let subject = parts[2]
        .parse::<i32>()
        .map_err(|_| crate::types::error::Error::Internal)?;
    let paper: Option<i16> = if parts[3].is_empty() {
        None
    } else {
        Some(
            parts[3]
                .parse::<i16>()
                .map_err(|_| crate::types::error::Error::Internal)?,
        )
    };
    let grade: i16 = parts[4]
        .parse::<i16>()
        .map_err(|_| crate::types::error::Error::Internal)?;
    let stream: Option<i16> = if parts[5].is_empty() {
        None
    } else {
        Some(
            parts[5]
                .parse::<i16>()
                .map_err(|_| crate::types::error::Error::Internal)?,
        )
    };
    sql_query(
        "UPDATE papers SET \
         topic = COALESCE(?, topic), \
         invigilator = COALESCE(?, invigilator), \
         start = COALESCE(?, start), \
         \"end\" = COALESCE(?, \"end\"), \
         status = COALESCE(?, status), \
         time_allowed_minutes = COALESCE(?, time_allowed_minutes), \
         instructions = COALESCE(?, instructions), \
         updated = ? \
         WHERE school = ? AND exam = ? AND subject = ? AND paper IS ? AND grade = ? AND stream IS ?",
    )
    .bind::<Nullable<Integer>, _>(row.topic)
    .bind::<Nullable<Text>, _>(row.invigilator.as_deref())
    .bind::<Nullable<BigInt>, _>(row.start)
    .bind::<Nullable<BigInt>, _>(row.end)
    .bind::<Nullable<SmallInt>, _>(row.status.map(|v| v as i16))
    .bind::<Nullable<SmallInt>, _>(row.time_allowed_minutes.map(|v| v as i16))
    .bind::<Nullable<Text>, _>(row.instructions.as_deref())
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(school)
    .bind::<Text, _>(exam)
    .bind::<Integer, _>(subject)
    .bind::<Nullable<SmallInt>, _>(paper)
    .bind::<SmallInt, _>(grade)
    .bind::<Nullable<SmallInt>, _>(stream)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// grades (PK: school|exam|student|subject|paper)
// paper is nullable in PK
// ---------------------------------------------------------------------------

pub fn update_grade(conn: &mut Conn, row_key: &str, row: &UpdateGradePayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let parts: Vec<&str> = row_key.split('|').collect();
    let school = parts[0];
    let exam = parts[1];
    let student = parts[2]
        .parse::<i32>()
        .map_err(|_| crate::types::error::Error::Internal)?;
    let subject = parts[3]
        .parse::<i16>()
        .map_err(|_| crate::types::error::Error::Internal)?;
    let paper: Option<i16> = if parts[4].is_empty() {
        None
    } else {
        Some(
            parts[4]
                .parse::<i16>()
                .map_err(|_| crate::types::error::Error::Internal)?,
        )
    };
    sql_query(
        "UPDATE grades SET \
         score = COALESCE(?, score), \
         total = COALESCE(?, total), \
         updated = ? \
         WHERE school = ? AND exam = ? AND student = ? AND subject = ? AND paper IS ?",
    )
    .bind::<Nullable<Float>, _>(row.score)
    .bind::<Nullable<Integer>, _>(row.total)
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(school)
    .bind::<Text, _>(exam)
    .bind::<Integer, _>(student)
    .bind::<SmallInt, _>(subject)
    .bind::<Nullable<SmallInt>, _>(paper)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// fees (PK: id)
// ---------------------------------------------------------------------------

pub fn update_fee(conn: &mut Conn, row_key: &str, row: &UpdateFeePayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "UPDATE fees SET \
         title = COALESCE(?, title), \
         description = COALESCE(?, description), \
         amount = COALESCE(?, amount), \
         mandatory = COALESCE(?, mandatory), \
         due = COALESCE(?, due), \
         updated = ? \
         WHERE id = ?",
    )
    .bind::<Nullable<Text>, _>(row.title.as_deref())
    .bind::<Nullable<Text>, _>(row.description.as_deref())
    .bind::<Nullable<Float>, _>(row.amount)
    .bind::<Nullable<Bool>, _>(row.mandatory)
    .bind::<Nullable<BigInt>, _>(row.due)
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(row_key)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// invoices (PK: id)
// ---------------------------------------------------------------------------

pub fn update_invoice(conn: &mut Conn, row_key: &str, row: &UpdateInvoicePayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "UPDATE invoices SET \
         fee = COALESCE(?, fee), \
         description = COALESCE(?, description), \
         amount = COALESCE(?, amount), \
         status = COALESCE(?, status), \
         due = COALESCE(?, due), \
         updated = ? \
         WHERE id = ?",
    )
    .bind::<Nullable<Text>, _>(row.fee.as_deref())
    .bind::<Nullable<Text>, _>(row.description.as_deref())
    .bind::<Nullable<Float>, _>(row.amount)
    .bind::<Nullable<SmallInt>, _>(row.status.map(|v| v as i16))
    .bind::<Nullable<BigInt>, _>(row.due)
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(row_key)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// payments (PK: id)
// ---------------------------------------------------------------------------

pub fn update_payment(conn: &mut Conn, row_key: &str, row: &UpdatePaymentPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "UPDATE payments SET \
         invoice = COALESCE(?, invoice), \
         amount = COALESCE(?, amount), \
         method = COALESCE(?, method), \
         reference = COALESCE(?, reference), \
         recorder = COALESCE(?, recorder), \
         date = COALESCE(?, date), \
         updated = ? \
         WHERE id = ?",
    )
    .bind::<Nullable<Text>, _>(row.invoice.as_deref())
    .bind::<Nullable<Float>, _>(row.amount)
    .bind::<Nullable<SmallInt>, _>(row.method.map(|v| v as i16))
    .bind::<Nullable<Text>, _>(row.reference.as_deref())
    .bind::<Nullable<Text>, _>(row.recorder.as_deref())
    .bind::<Nullable<Integer>, _>(row.date)
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(row_key)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// announcements (PK: id)
// ---------------------------------------------------------------------------

pub fn update_announcement(
    conn: &mut Conn,
    row_key: &str,
    row: &UpdateAnnouncementPayload,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "UPDATE announcements SET \
         title = COALESCE(?, title), \
         content = COALESCE(?, content), \
         grade = COALESCE(?, grade), \
         stream = COALESCE(?, stream), \
         audience = COALESCE(?, audience), \
         updated = ? \
         WHERE id = ?",
    )
    .bind::<Nullable<Text>, _>(row.title.as_deref())
    .bind::<Nullable<Text>, _>(row.content.as_deref())
    .bind::<Nullable<SmallInt>, _>(row.grade.map(|v| v as i16))
    .bind::<Nullable<SmallInt>, _>(row.stream.map(|v| v as i16))
    .bind::<Nullable<Integer>, _>(row.audience)
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(row_key)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// mastery (PK: school|student|grade|subject|topic)
// ---------------------------------------------------------------------------

pub fn update_mastery(conn: &mut Conn, row_key: &str, row: &UpdateMasteryPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let parts: Vec<&str> = row_key.split('|').collect();
    let (school, student, subject, topic) = (
        parts[0],
        parts[1]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[2]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[3]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
    );
    sql_query(
        "UPDATE mastery SET \
         score = ?, \
         updated = ? \
         WHERE school = ? AND student = ? AND subject = ? AND topic = ?",
    )
    .bind::<Float, _>(row.score)
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(school)
    .bind::<Integer, _>(student)
    .bind::<Integer, _>(subject)
    .bind::<Integer, _>(topic)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// aiusage (PK: school|student|year|term)
// ---------------------------------------------------------------------------

pub fn update_ai_usage(conn: &mut Conn, row_key: &str, row: &UpdateAiUsagePayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let parts: Vec<&str> = row_key.split('|').collect();
    let (school, student, year, term) = (
        parts[0],
        parts[1]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[2]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[3]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
    );
    sql_query(
        "UPDATE aiusage SET \
         allocated = COALESCE(?, allocated), \
         used = COALESCE(?, used), \
         updated = ? \
         WHERE school = ? AND student = ? AND year = ? AND term = ?",
    )
    .bind::<Nullable<Integer>, _>(row.allocated)
    .bind::<Nullable<Integer>, _>(row.used)
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(school)
    .bind::<Integer, _>(student)
    .bind::<Integer, _>(year)
    .bind::<SmallInt, _>(term)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// roles (PK: id)
// ---------------------------------------------------------------------------

pub fn update_role(conn: &mut Conn, row_key: &str, row: &UpdateRolePayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "UPDATE roles SET \
         name = COALESCE(?, name), \
         description = COALESCE(?, description), \
         permissions = COALESCE(?, permissions), \
         updated = ? \
         WHERE id = ?",
    )
    .bind::<Nullable<Text>, _>(row.name.as_deref())
    .bind::<Nullable<Text>, _>(row.description.as_deref())
    .bind::<Nullable<Binary>, _>(row.permissions.as_deref())
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(row_key)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// plans (PK: id)
// ---------------------------------------------------------------------------

pub fn update_plan(conn: &mut Conn, row_key: &str, row: &UpdatePlanPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "UPDATE plans SET \
         name = COALESCE(?, name), \
         description = COALESCE(?, description), \
         amount = COALESCE(?, amount), \
         levels = COALESCE(?, levels), \
         status = COALESCE(?, status), \
         features = COALESCE(?, features), \
         updated = ? \
         WHERE id = ?",
    )
    .bind::<Nullable<Text>, _>(row.name.as_deref())
    .bind::<Nullable<Text>, _>(row.description.as_deref())
    .bind::<Nullable<Float>, _>(row.amount)
    .bind::<Nullable<Integer>, _>(row.levels)
    .bind::<Nullable<SmallInt>, _>(row.status.map(|v| v as i16))
    .bind::<Nullable<Text>, _>(row.features.as_deref())
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(row_key)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// subscriptions (PK: school|plan|year|term|student)
// ---------------------------------------------------------------------------

pub fn update_subscription(
    conn: &mut Conn,
    row_key: &str,
    row: &UpdateSubscriptionPayload,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let parts: Vec<&str> = row_key.split('|').collect();
    let (school, plan, year, term, student) = (
        parts[0],
        parts[1],
        parts[2]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[3]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[4]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
    );
    sql_query(
        "UPDATE subscriptions SET \
         invoice = COALESCE(?, invoice), \
         discount = COALESCE(?, discount), \
         status = COALESCE(?, status), \
         updated = ? \
         WHERE school = ? AND plan = ? AND year = ? AND term = ? AND student = ?",
    )
    .bind::<Nullable<Text>, _>(row.invoice.as_deref())
    .bind::<Nullable<Float>, _>(row.discount)
    .bind::<Nullable<SmallInt>, _>(row.status.map(|v| v as i16))
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(school)
    .bind::<Text, _>(plan)
    .bind::<Integer, _>(year)
    .bind::<SmallInt, _>(term)
    .bind::<Integer, _>(student)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// subjects catalog (PK: id)
// ---------------------------------------------------------------------------

pub fn update_subject_catalog(conn: &mut Conn, id: i32, row: &UpdateSubjectPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "UPDATE subjects SET \
         name = COALESCE(?, name), \
         curriculum = COALESCE(?, curriculum), \
         updated = ? \
         WHERE id = ?",
    )
    .bind::<Nullable<Text>, _>(row.name.as_deref())
    .bind::<Nullable<SmallInt>, _>(row.curriculum.map(|v| v as i16))
    .bind::<BigInt, _>(now)
    .bind::<Integer, _>(id)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// topics (PK: id)
// ---------------------------------------------------------------------------

pub fn update_topic(conn: &mut Conn, id: i32, row: &UpdateTopicPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "UPDATE topics SET \
         subject = COALESCE(?, subject), \
         grade = COALESCE(?, grade), \
         name = COALESCE(?, name), \
         updated = ? \
         WHERE id = ?",
    )
    .bind::<Nullable<Integer>, _>(row.subject)
    .bind::<Nullable<SmallInt>, _>(row.grade.map(|v| v as i16))
    .bind::<Nullable<Text>, _>(row.name.as_deref())
    .bind::<BigInt, _>(now)
    .bind::<Integer, _>(id)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// streams (PK: school|grade|stream)
// ---------------------------------------------------------------------------

pub fn update_stream(conn: &mut Conn, row: &UpdateStreamPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "UPDATE streams SET \
         name = COALESCE(?, name), \
         updated = ? \
         WHERE school = ? AND grade = ? AND stream = ?",
    )
    .bind::<Nullable<Text>, _>(row.name.as_deref())
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(&row.school)
    .bind::<SmallInt, _>(row.grade as i16)
    .bind::<SmallInt, _>(row.stream as i16)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// mpesa (PK: school)
// ---------------------------------------------------------------------------

pub fn update_mpesa(conn: &mut Conn, row: &UpdateMpesaPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "UPDATE mpesa SET \
         consumer_key = COALESCE(?, consumer_key), \
         consumer_secret = COALESCE(?, consumer_secret), \
         passkey = COALESCE(?, passkey), \
         shortcode = COALESCE(?, shortcode), \
         env = COALESCE(?, env), \
         updated = ? \
         WHERE school = ?",
    )
    .bind::<Nullable<Text>, _>(row.consumer_key.as_deref())
    .bind::<Nullable<Text>, _>(row.consumer_secret.as_deref())
    .bind::<Nullable<Text>, _>(row.passkey.as_deref())
    .bind::<Nullable<Text>, _>(row.shortcode.as_deref())
    .bind::<Nullable<SmallInt>, _>(row.env.map(|v| v as i16))
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(&row.school)
    .execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// discounts (PK: school|plan|year|term|grade)
// ---------------------------------------------------------------------------

pub fn update_discount(conn: &mut Conn, row_key: &str, row: &UpdateDiscountPayload) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let parts: Vec<&str> = row_key.split('|').collect();
    let (school, plan, year, term, grade) = (
        parts[0],
        parts[1],
        parts[2]
            .parse::<i32>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[3]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
        parts[4]
            .parse::<i16>()
            .map_err(|_| crate::types::error::Error::Internal)?,
    );
    sql_query(
        "UPDATE discounts SET \
         amount = COALESCE(?, amount), \
         unit = COALESCE(?, unit), \
         updated = ? \
         WHERE school = ? AND plan = ? AND year = ? AND term = ? AND grade = ?",
    )
    .bind::<Nullable<Float>, _>(row.amount)
    .bind::<Nullable<SmallInt>, _>(row.unit.map(|v| v as i16))
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(school)
    .bind::<Text, _>(plan)
    .bind::<Integer, _>(year)
    .bind::<SmallInt, _>(term)
    .bind::<SmallInt, _>(grade)
    .execute(conn)?;
    Ok(())
}
