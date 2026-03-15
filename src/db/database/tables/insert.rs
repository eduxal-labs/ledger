use crate::proto::services::sync::*;
use crate::types::error::Result;
use diesel::RunQueryDsl;
use diesel::SqliteConnection as Conn;
use diesel::sql_query;
use diesel::sql_types::{BigInt, Binary, Bool, Float, Integer, Nullable, SmallInt, Text};

pub fn insert_user(conn: &mut Conn, row: &UserInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO users (id, phone, email, name, level, status, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.id)
    .bind::<Text, _>(&row.phone)
    .bind::<Nullable<Text>, _>(row.email.as_deref())
    .bind::<Text, _>(&row.name)
    .bind::<SmallInt, _>(row.level as i16)
    .bind::<SmallInt, _>(row.status as i16)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_school(conn: &mut Conn, row: &SchoolInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO schools (id, name, motto, phone, email, county, domain, established, status, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.id)
    .bind::<Text, _>(&row.name)
    .bind::<Nullable<Text>, _>(row.motto.as_deref())
    .bind::<Nullable<Text>, _>(row.phone.as_deref())
    .bind::<Nullable<Text>, _>(row.email.as_deref())
    .bind::<Integer, _>(row.county)
    .bind::<Nullable<Text>, _>(row.domain.as_deref())
    .bind::<Nullable<Integer>, _>(row.established)
    .bind::<SmallInt, _>(row.status as i16)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_owner(conn: &mut Conn, row: &OwnerInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query("INSERT INTO owners (school, user, created) VALUES (?, ?, ?)")
        .bind::<Text, _>(&row.school)
        .bind::<Text, _>(&row.user)
        .bind::<BigInt, _>(now)
        .execute(conn)?;
    Ok(())
}

pub fn insert_student(conn: &mut Conn, row: &StudentInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO students (school, adm, user, name, dob, gender, documents, admitted, status, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Integer, _>(row.adm)
    .bind::<Nullable<Text>, _>(row.user.as_deref())
    .bind::<Text, _>(&row.name)
    .bind::<Nullable<Integer>, _>(row.dob)
    .bind::<Nullable<SmallInt>, _>(row.gender.map(|v| v as i16))
    .bind::<Nullable<Text>, _>(row.documents.as_deref())
    .bind::<Nullable<Integer>, _>(row.admitted)
    .bind::<SmallInt, _>(row.status as i16)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_guardian(conn: &mut Conn, row: &GuardianInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO guardians (school, user, student, relationship, role, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Text, _>(&row.user)
    .bind::<Integer, _>(row.student)
    .bind::<SmallInt, _>(row.relationship as i16)
    .bind::<SmallInt, _>(row.role as i16)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_department(conn: &mut Conn, row: &DepartmentInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO departments (school, name, description, created, updated) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Text, _>(&row.name)
    .bind::<Nullable<Text>, _>(row.description.as_deref())
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_teacher(conn: &mut Conn, row: &TeacherInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO teachers (school, user, hired, role, department, status, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Text, _>(&row.user)
    .bind::<Nullable<Integer>, _>(row.hired)
    .bind::<Nullable<Text>, _>(row.role.as_deref())
    .bind::<Nullable<Text>, _>(row.department.as_deref())
    .bind::<SmallInt, _>(row.status as i16)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_staff(conn: &mut Conn, row: &StaffInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO staff (school, user, idnumber, role, department, status, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Text, _>(&row.user)
    .bind::<Nullable<Text>, _>(row.idnumber.as_deref())
    .bind::<Nullable<Text>, _>(row.role.as_deref())
    .bind::<Nullable<Text>, _>(row.department.as_deref())
    .bind::<SmallInt, _>(row.status as i16)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_term(conn: &mut Conn, row: &TermInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO terms (school, year, term, start, end, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Integer, _>(row.year)
    .bind::<SmallInt, _>(row.term as i16)
    .bind::<BigInt, _>(row.start)
    .bind::<BigInt, _>(row.end)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_class_teacher(conn: &mut Conn, row: &ClassTeacherInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO class_teachers (school, year, term, grade, stream, teacher, start, \"end\", created) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Integer, _>(row.year)
    .bind::<SmallInt, _>(row.term as i16)
    .bind::<SmallInt, _>(row.grade as i16)
    .bind::<SmallInt, _>(row.stream as i16)
    .bind::<Text, _>(&row.teacher)
    .bind::<Integer, _>(row.start)
    .bind::<Nullable<Integer>, _>(row.end)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_enrollment(conn: &mut Conn, row: &EnrollmentInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO enrollments (school, year, term, grade, stream, student, created) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Integer, _>(row.year)
    .bind::<SmallInt, _>(row.term as i16)
    .bind::<SmallInt, _>(row.grade as i16)
    .bind::<SmallInt, _>(row.stream as i16)
    .bind::<Integer, _>(row.student)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_subject_teacher(conn: &mut Conn, row: &SubjectTeacherInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO subject_teachers (school, year, term, grade, stream, subject, teacher, created) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Integer, _>(row.year)
    .bind::<SmallInt, _>(row.term as i16)
    .bind::<SmallInt, _>(row.grade as i16)
    .bind::<SmallInt, _>(row.stream as i16)
    .bind::<Integer, _>(row.subject)
    .bind::<Text, _>(&row.teacher)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_attendance(conn: &mut Conn, row: &AttendanceInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO attendance (school, year, term, grade, stream, student, date, status, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Integer, _>(row.year)
    .bind::<SmallInt, _>(row.term as i16)
    .bind::<SmallInt, _>(row.grade as i16)
    .bind::<SmallInt, _>(row.stream as i16)
    .bind::<Integer, _>(row.student)
    .bind::<Integer, _>(row.date)
    .bind::<SmallInt, _>(row.status as i16)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_timetable(conn: &mut Conn, row: &TimetableInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO timetable (school, year, term, grade, stream, subject, teacher, day, start, \"end\", created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Integer, _>(row.year)
    .bind::<SmallInt, _>(row.term as i16)
    .bind::<SmallInt, _>(row.grade as i16)
    .bind::<SmallInt, _>(row.stream as i16)
    .bind::<SmallInt, _>(row.subject as i16)
    .bind::<Text, _>(&row.teacher)
    .bind::<SmallInt, _>(row.day as i16)
    .bind::<Integer, _>(row.start)
    .bind::<Integer, _>(row.end)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_lesson(conn: &mut Conn, row: &LessonInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO lessons (school, year, term, grade, stream, date, subject, teacher, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Integer, _>(row.year)
    .bind::<SmallInt, _>(row.term as i16)
    .bind::<SmallInt, _>(row.grade as i16)
    .bind::<SmallInt, _>(row.stream as i16)
    .bind::<Integer, _>(row.date)
    .bind::<SmallInt, _>(row.subject as i16)
    .bind::<Text, _>(&row.teacher)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_exam(conn: &mut Conn, row: &ExamInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO exams (id, school, name, year, term, personalized, type, start, \"end\", teacher, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.id)
    .bind::<Text, _>(&row.school)
    .bind::<Text, _>(&row.name)
    .bind::<Integer, _>(row.year)
    .bind::<SmallInt, _>(row.term as i16)
    .bind::<Bool, _>(row.personalized)
    .bind::<SmallInt, _>(row.r#type as i16)
    .bind::<Integer, _>(row.start)
    .bind::<Integer, _>(row.end)
    .bind::<Text, _>(&row.teacher)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_paper(conn: &mut Conn, row: &PaperInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO papers (school, exam, subject, paper, topic, invigilator, start, \"end\", status, grade, stream, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Text, _>(&row.exam)
    .bind::<Integer, _>(row.subject)
    .bind::<Nullable<SmallInt>, _>(row.paper.map(|v| v as i16))
    .bind::<Nullable<Integer>, _>(row.topic)
    .bind::<Text, _>(&row.invigilator)
    .bind::<BigInt, _>(row.start)
    .bind::<BigInt, _>(row.end)
    .bind::<SmallInt, _>(row.status as i16)
    .bind::<SmallInt, _>(row.grade as i16)
    .bind::<Nullable<SmallInt>, _>(row.stream.map(|v| v as i16))
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_grade(conn: &mut Conn, row: &GradeInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO grades (school, exam, student, subject, paper, score, total, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Text, _>(&row.exam)
    .bind::<Integer, _>(row.student)
    .bind::<SmallInt, _>(row.subject as i16)
    .bind::<Nullable<SmallInt>, _>(row.paper.map(|v| v as i16))
    .bind::<Float, _>(row.score)
    .bind::<Integer, _>(row.total)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_fee(conn: &mut Conn, row: &FeeInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO fees (id, school, year, term, grade, title, description, amount, mandatory, due, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.id)
    .bind::<Text, _>(&row.school)
    .bind::<Integer, _>(row.year)
    .bind::<SmallInt, _>(row.term as i16)
    .bind::<SmallInt, _>(row.grade as i16)
    .bind::<Text, _>(&row.title)
    .bind::<Text, _>(&row.description)
    .bind::<Float, _>(row.amount)
    .bind::<Bool, _>(row.mandatory)
    .bind::<BigInt, _>(row.due)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_invoice(conn: &mut Conn, row: &InvoiceInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO invoices (id, school, year, term, fee, description, student, amount, status, due, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.id)
    .bind::<Text, _>(&row.school)
    .bind::<Integer, _>(row.year)
    .bind::<SmallInt, _>(row.term as i16)
    .bind::<Nullable<Text>, _>(row.fee.as_deref())
    .bind::<Nullable<Text>, _>(row.description.as_deref())
    .bind::<Integer, _>(row.student)
    .bind::<Float, _>(row.amount)
    .bind::<SmallInt, _>(row.status as i16)
    .bind::<Nullable<BigInt>, _>(row.due)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_payment(conn: &mut Conn, row: &PaymentInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO payments (id, invoice, school, student, amount, method, reference, recorder, date, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.id)
    .bind::<Nullable<Text>, _>(row.invoice.as_deref())
    .bind::<Nullable<Text>, _>(row.school.as_deref())
    .bind::<Nullable<Integer>, _>(row.student)
    .bind::<Float, _>(row.amount)
    .bind::<SmallInt, _>(row.method as i16)
    .bind::<Nullable<Text>, _>(row.reference.as_deref())
    .bind::<Nullable<Text>, _>(row.recorder.as_deref())
    .bind::<Nullable<Integer>, _>(row.date)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_announcement(conn: &mut Conn, row: &AnnouncementInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO announcements (id, school, title, content, grade, stream, audience, author, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.id)
    .bind::<Text, _>(&row.school)
    .bind::<Text, _>(&row.title)
    .bind::<Text, _>(&row.content)
    .bind::<Nullable<SmallInt>, _>(row.grade.map(|v| v as i16))
    .bind::<Nullable<SmallInt>, _>(row.stream.map(|v| v as i16))
    .bind::<Integer, _>(row.audience)
    .bind::<Nullable<Text>, _>(row.author.as_deref())
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_mastery(conn: &mut Conn, row: &MasteryInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO mastery (school, student, subject, topic, score, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Integer, _>(row.student)
    .bind::<Integer, _>(row.subject)
    .bind::<Integer, _>(row.topic)
    .bind::<Float, _>(row.score)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_ai_usage(conn: &mut Conn, row: &AiUsageInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO aiusage (school, student, year, term, allocated, used, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Integer, _>(row.student)
    .bind::<Integer, _>(row.year)
    .bind::<SmallInt, _>(row.term as i16)
    .bind::<Integer, _>(row.allocated)
    .bind::<Integer, _>(row.used)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_role(conn: &mut Conn, row: &RoleInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO roles (id, school, name, description, permissions, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.id)
    .bind::<Nullable<Text>, _>(row.school.as_deref())
    .bind::<Text, _>(&row.name)
    .bind::<Nullable<Text>, _>(row.description.as_deref())
    .bind::<Binary, _>(&row.permissions)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_scope(conn: &mut Conn, row: &ScopeInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO scopes (school, user, role, created) \
         VALUES (?, ?, ?, ?)",
    )
    .bind::<Nullable<Text>, _>(row.school.as_deref())
    .bind::<Text, _>(&row.user)
    .bind::<Text, _>(&row.role)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_plan(conn: &mut Conn, row: &PlanInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO plans (id, name, description, amount, levels, status, features, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.id)
    .bind::<Text, _>(&row.name)
    .bind::<Nullable<Text>, _>(row.description.as_deref())
    .bind::<Float, _>(row.amount)
    .bind::<Integer, _>(row.levels)
    .bind::<SmallInt, _>(row.status as i16)
    .bind::<Nullable<Text>, _>(row.features.as_deref())
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_subscription(conn: &mut Conn, row: &SubscriptionInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO subscriptions (school, plan, year, term, student, invoice, discount, status, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Text, _>(&row.plan)
    .bind::<Integer, _>(row.year)
    .bind::<SmallInt, _>(row.term as i16)
    .bind::<Integer, _>(row.student)
    .bind::<Nullable<Text>, _>(row.invoice.as_deref())
    .bind::<Float, _>(row.discount)
    .bind::<SmallInt, _>(row.status as i16)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_subject_catalog(conn: &mut Conn, row: &SubjectInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT OR IGNORE INTO subjects (id, name, curriculum, created, updated) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind::<Integer, _>(row.id)
    .bind::<Text, _>(&row.name)
    .bind::<SmallInt, _>(row.curriculum as i16)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_topic(conn: &mut Conn, row: &TopicInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT OR IGNORE INTO topics (id, subject, grade, name, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind::<Integer, _>(row.id)
    .bind::<Integer, _>(row.subject)
    .bind::<SmallInt, _>(row.grade as i16)
    .bind::<Text, _>(&row.name)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_stream(conn: &mut Conn, row: &StreamInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO streams (school, grade, stream, name, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<SmallInt, _>(row.grade as i16)
    .bind::<SmallInt, _>(row.stream as i16)
    .bind::<Text, _>(&row.name)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_mpesa(conn: &mut Conn, row: &MpesaInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO mpesa (school, consumer_key, consumer_secret, passkey, shortcode, env, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Text, _>(&row.consumer_key)
    .bind::<Text, _>(&row.consumer_secret)
    .bind::<Text, _>(&row.passkey)
    .bind::<Text, _>(&row.shortcode)
    .bind::<SmallInt, _>(row.env as i16)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

pub fn insert_discount(conn: &mut Conn, row: &DiscountInsert) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO discounts (school, plan, year, term, grade, amount, unit, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(&row.school)
    .bind::<Text, _>(&row.plan)
    .bind::<Integer, _>(row.year)
    .bind::<SmallInt, _>(row.term as i16)
    .bind::<SmallInt, _>(row.grade as i16)
    .bind::<Float, _>(row.amount)
    .bind::<SmallInt, _>(row.unit as i16)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}
