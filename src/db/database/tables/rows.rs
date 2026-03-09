use crate::proto::services::sync::*;
use diesel::QueryableByName;
use diesel::sql_types::{BigInt, Binary, Bool, Float, Integer, Nullable, SmallInt, Text};

// ---------------------------------------------------------------------------
// 1. Users
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct UserRow {
    #[diesel(sql_type = Text)]
    pub id: String,
    #[diesel(sql_type = Text)]
    pub phone: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub email: Option<String>,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = SmallInt)]
    pub level: i16,
    #[diesel(sql_type = SmallInt)]
    pub status: i16,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl UserRow {
    pub fn row_key(&self) -> String {
        self.id.clone()
    }
    pub fn school_id(&self) -> Option<&str> {
        None
    }
}

impl From<&UserRow> for UserInsert {
    fn from(row: &UserRow) -> Self {
        UserInsert {
            id: row.id.clone(),
            phone: row.phone.clone(),
            email: row.email.clone(),
            name: row.name.clone(),
            level: row.level as i32,
            status: row.status as i32,
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Schools
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct SchoolRow {
    #[diesel(sql_type = Text)]
    pub id: String,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub motto: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub phone: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub email: Option<String>,
    #[diesel(sql_type = Integer)]
    pub county: i32,
    #[diesel(sql_type = Nullable<Text>)]
    pub domain: Option<String>,
    #[diesel(sql_type = Nullable<Integer>)]
    pub established: Option<i32>,
    #[diesel(sql_type = SmallInt)]
    pub status: i16,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl SchoolRow {
    pub fn row_key(&self) -> String {
        self.id.clone()
    }
    pub fn school_id(&self) -> Option<&str> {
        None
    }
}

impl From<&SchoolRow> for SchoolInsert {
    fn from(row: &SchoolRow) -> Self {
        SchoolInsert {
            id: row.id.clone(),
            name: row.name.clone(),
            motto: row.motto.clone(),
            phone: row.phone.clone(),
            email: row.email.clone(),
            county: row.county,
            domain: row.domain.clone(),
            established: row.established,
            status: row.status as i32,
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Owners
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct OwnerRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Text)]
    pub user: String,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
}

impl OwnerRow {
    pub fn row_key(&self) -> String {
        format!("{}|{}", self.school, self.user)
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&OwnerRow> for OwnerInsert {
    fn from(row: &OwnerRow) -> Self {
        OwnerInsert {
            school: row.school.clone(),
            user: row.user.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Students
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct StudentRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Integer)]
    pub adm: i32,
    #[diesel(sql_type = Nullable<Text>)]
    pub user: Option<String>,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Nullable<Integer>)]
    pub dob: Option<i32>,
    #[diesel(sql_type = Nullable<SmallInt>)]
    pub gender: Option<i16>,
    #[diesel(sql_type = Nullable<Text>)]
    pub documents: Option<String>,
    #[diesel(sql_type = Nullable<Integer>)]
    pub admitted: Option<i32>,
    #[diesel(sql_type = SmallInt)]
    pub status: i16,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl StudentRow {
    pub fn row_key(&self) -> String {
        format!("{}|{}", self.school, self.adm)
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&StudentRow> for StudentInsert {
    fn from(row: &StudentRow) -> Self {
        StudentInsert {
            school: row.school.clone(),
            adm: row.adm,
            user: row.user.clone(),
            name: row.name.clone(),
            dob: row.dob,
            gender: row.gender.map(|v| v as i32),
            documents: row.documents.clone(),
            admitted: row.admitted,
            status: row.status as i32,
        }
    }
}

// ---------------------------------------------------------------------------
// 5. Guardians
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct GuardianRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Text)]
    pub user: String,
    #[diesel(sql_type = Integer)]
    pub student: i32,
    #[diesel(sql_type = SmallInt)]
    pub relationship: i16,
    #[diesel(sql_type = SmallInt)]
    pub role: i16,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl GuardianRow {
    pub fn row_key(&self) -> String {
        format!("{}|{}|{}", self.school, self.user, self.student)
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&GuardianRow> for GuardianInsert {
    fn from(row: &GuardianRow) -> Self {
        GuardianInsert {
            school: row.school.clone(),
            user: row.user.clone(),
            student: row.student,
            relationship: row.relationship as i32,
            role: row.role as i32,
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Departments
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct DepartmentRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub description: Option<String>,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl DepartmentRow {
    pub fn row_key(&self) -> String {
        format!("{}|{}", self.school, self.name)
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&DepartmentRow> for DepartmentInsert {
    fn from(row: &DepartmentRow) -> Self {
        DepartmentInsert {
            school: row.school.clone(),
            name: row.name.clone(),
            description: row.description.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// 7. Teachers
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct TeacherRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Text)]
    pub user: String,
    #[diesel(sql_type = Nullable<Integer>)]
    pub hired: Option<i32>,
    #[diesel(sql_type = Nullable<Text>)]
    pub role: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub department: Option<String>,
    #[diesel(sql_type = SmallInt)]
    pub status: i16,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl TeacherRow {
    pub fn row_key(&self) -> String {
        format!("{}|{}", self.school, self.user)
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&TeacherRow> for TeacherInsert {
    fn from(row: &TeacherRow) -> Self {
        TeacherInsert {
            school: row.school.clone(),
            user: row.user.clone(),
            hired: row.hired,
            role: row.role.clone(),
            department: row.department.clone(),
            status: row.status as i32,
        }
    }
}

// ---------------------------------------------------------------------------
// 8. Staff
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct StaffRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Text)]
    pub user: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub idnumber: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub role: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub department: Option<String>,
    #[diesel(sql_type = SmallInt)]
    pub status: i16,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl StaffRow {
    pub fn row_key(&self) -> String {
        format!("{}|{}", self.school, self.user)
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&StaffRow> for StaffInsert {
    fn from(row: &StaffRow) -> Self {
        StaffInsert {
            school: row.school.clone(),
            user: row.user.clone(),
            idnumber: row.idnumber.clone(),
            role: row.role.clone(),
            department: row.department.clone(),
            status: row.status as i32,
        }
    }
}

// ---------------------------------------------------------------------------
// 9. Terms
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct TermRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Integer)]
    pub year: i32,
    #[diesel(sql_type = SmallInt)]
    pub term: i16,
    #[diesel(sql_type = BigInt)]
    pub start: i64,
    #[diesel(sql_type = BigInt)]
    pub end: i64,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl TermRow {
    pub fn row_key(&self) -> String {
        format!("{}|{}|{}", self.school, self.year, self.term)
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&TermRow> for TermInsert {
    fn from(row: &TermRow) -> Self {
        TermInsert {
            school: row.school.clone(),
            year: row.year,
            term: row.term as i32,
            start: row.start,
            end: row.end,
        }
    }
}

// ---------------------------------------------------------------------------
// 10. ClassTeachers
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct ClassTeacherRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Integer)]
    pub year: i32,
    #[diesel(sql_type = SmallInt)]
    pub term: i16,
    #[diesel(sql_type = SmallInt)]
    pub grade: i16,
    #[diesel(sql_type = SmallInt)]
    pub stream: i16,
    #[diesel(sql_type = Text)]
    pub teacher: String,
    #[diesel(sql_type = Integer)]
    pub start: i32,
    #[diesel(sql_type = Nullable<Integer>)]
    pub end: Option<i32>,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
}

impl ClassTeacherRow {
    pub fn row_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}",
            self.school, self.year, self.term, self.grade, self.stream, self.teacher
        )
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&ClassTeacherRow> for ClassTeacherInsert {
    fn from(row: &ClassTeacherRow) -> Self {
        ClassTeacherInsert {
            school: row.school.clone(),
            year: row.year,
            term: row.term as i32,
            grade: row.grade as i32,
            stream: row.stream as i32,
            teacher: row.teacher.clone(),
            start: row.start,
            end: row.end,
        }
    }
}

// ---------------------------------------------------------------------------
// 11. Enrollments
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct EnrollmentRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Integer)]
    pub year: i32,
    #[diesel(sql_type = SmallInt)]
    pub term: i16,
    #[diesel(sql_type = SmallInt)]
    pub grade: i16,
    #[diesel(sql_type = SmallInt)]
    pub stream: i16,
    #[diesel(sql_type = Integer)]
    pub student: i32,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
}

impl EnrollmentRow {
    pub fn row_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}",
            self.school, self.year, self.term, self.grade, self.stream, self.student
        )
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&EnrollmentRow> for EnrollmentInsert {
    fn from(row: &EnrollmentRow) -> Self {
        EnrollmentInsert {
            school: row.school.clone(),
            year: row.year,
            term: row.term as i32,
            grade: row.grade as i32,
            stream: row.stream as i32,
            student: row.student,
        }
    }
}

// ---------------------------------------------------------------------------
// 12. Subjects
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct SubjectRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Integer)]
    pub year: i32,
    #[diesel(sql_type = SmallInt)]
    pub term: i16,
    #[diesel(sql_type = SmallInt)]
    pub grade: i16,
    #[diesel(sql_type = SmallInt)]
    pub stream: i16,
    #[diesel(sql_type = SmallInt)]
    pub subject: i16,
    #[diesel(sql_type = Text)]
    pub teacher: String,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
}

impl SubjectRow {
    pub fn row_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}",
            self.school, self.year, self.term, self.grade, self.stream, self.subject
        )
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&SubjectRow> for SubjectInsert {
    fn from(row: &SubjectRow) -> Self {
        SubjectInsert {
            school: row.school.clone(),
            year: row.year,
            term: row.term as i32,
            grade: row.grade as i32,
            stream: row.stream as i32,
            subject: row.subject as i32,
            teacher: row.teacher.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// 13. Attendance
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct AttendanceRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Integer)]
    pub year: i32,
    #[diesel(sql_type = SmallInt)]
    pub term: i16,
    #[diesel(sql_type = SmallInt)]
    pub grade: i16,
    #[diesel(sql_type = SmallInt)]
    pub stream: i16,
    #[diesel(sql_type = Integer)]
    pub student: i32,
    #[diesel(sql_type = Integer)]
    pub date: i32,
    #[diesel(sql_type = SmallInt)]
    pub status: i16,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl AttendanceRow {
    pub fn row_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}",
            self.school, self.year, self.term, self.grade, self.stream, self.student, self.date
        )
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&AttendanceRow> for AttendanceInsert {
    fn from(row: &AttendanceRow) -> Self {
        AttendanceInsert {
            school: row.school.clone(),
            year: row.year,
            term: row.term as i32,
            grade: row.grade as i32,
            stream: row.stream as i32,
            student: row.student,
            date: row.date,
            status: row.status as i32,
        }
    }
}

// ---------------------------------------------------------------------------
// 14. Timetable
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct TimetableRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Integer)]
    pub year: i32,
    #[diesel(sql_type = SmallInt)]
    pub term: i16,
    #[diesel(sql_type = SmallInt)]
    pub grade: i16,
    #[diesel(sql_type = SmallInt)]
    pub stream: i16,
    #[diesel(sql_type = SmallInt)]
    pub subject: i16,
    #[diesel(sql_type = Text)]
    pub teacher: String,
    #[diesel(sql_type = SmallInt)]
    pub day: i16,
    #[diesel(sql_type = Integer)]
    pub start: i32,
    #[diesel(sql_type = Integer)]
    pub end: i32,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl TimetableRow {
    pub fn row_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            self.school,
            self.year,
            self.term,
            self.grade,
            self.stream,
            self.subject,
            self.day,
            self.start
        )
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&TimetableRow> for TimetableInsert {
    fn from(row: &TimetableRow) -> Self {
        TimetableInsert {
            school: row.school.clone(),
            year: row.year,
            term: row.term as i32,
            grade: row.grade as i32,
            stream: row.stream as i32,
            subject: row.subject as i32,
            teacher: row.teacher.clone(),
            day: row.day as i32,
            start: row.start,
            end: row.end,
        }
    }
}

// ---------------------------------------------------------------------------
// 15. Lessons
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct LessonRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Integer)]
    pub year: i32,
    #[diesel(sql_type = SmallInt)]
    pub term: i16,
    #[diesel(sql_type = SmallInt)]
    pub grade: i16,
    #[diesel(sql_type = SmallInt)]
    pub stream: i16,
    #[diesel(sql_type = Integer)]
    pub date: i32,
    #[diesel(sql_type = SmallInt)]
    pub subject: i16,
    #[diesel(sql_type = Text)]
    pub teacher: String,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl LessonRow {
    pub fn row_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            self.school,
            self.year,
            self.term,
            self.grade,
            self.stream,
            self.date,
            self.subject,
            self.teacher
        )
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&LessonRow> for LessonInsert {
    fn from(row: &LessonRow) -> Self {
        LessonInsert {
            school: row.school.clone(),
            year: row.year,
            term: row.term as i32,
            grade: row.grade as i32,
            stream: row.stream as i32,
            date: row.date,
            subject: row.subject as i32,
            teacher: row.teacher.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// 16. Exams
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct ExamRow {
    #[diesel(sql_type = Text)]
    pub id: String,
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Integer)]
    pub year: i32,
    #[diesel(sql_type = SmallInt)]
    pub term: i16,
    #[diesel(sql_type = SmallInt)]
    pub grade: i16,
    #[diesel(sql_type = Nullable<SmallInt>)]
    pub stream: Option<i16>,
    #[diesel(sql_type = Bool)]
    pub personalized: bool,
    #[diesel(sql_type = SmallInt, column_name = "type")]
    pub type_: i16,
    #[diesel(sql_type = Integer)]
    pub start: i32,
    #[diesel(sql_type = Integer)]
    pub end: i32,
    #[diesel(sql_type = Text)]
    pub teacher: String,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl ExamRow {
    pub fn row_key(&self) -> String {
        self.id.clone()
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&ExamRow> for ExamInsert {
    fn from(row: &ExamRow) -> Self {
        ExamInsert {
            id: row.id.clone(),
            school: row.school.clone(),
            year: row.year,
            term: row.term as i32,
            grade: row.grade as i32,
            stream: row.stream.map(|v| v as i32),
            personalized: row.personalized,
            r#type: row.type_ as i32,
            start: row.start,
            end: row.end,
            teacher: row.teacher.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// 17. Papers
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct PaperRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Text)]
    pub exam: String,
    #[diesel(sql_type = SmallInt)]
    pub subject: i16,
    #[diesel(sql_type = Nullable<SmallInt>)]
    pub paper: Option<i16>,
    #[diesel(sql_type = Text)]
    pub invigilator: String,
    #[diesel(sql_type = BigInt)]
    pub start: i64,
    #[diesel(sql_type = BigInt)]
    pub end: i64,
    #[diesel(sql_type = SmallInt)]
    pub status: i16,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl PaperRow {
    pub fn row_key(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.school,
            self.exam,
            self.subject,
            self.paper.map(|v| v.to_string()).unwrap_or_default()
        )
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&PaperRow> for PaperInsert {
    fn from(row: &PaperRow) -> Self {
        PaperInsert {
            school: row.school.clone(),
            exam: row.exam.clone(),
            subject: row.subject as i32,
            paper: row.paper.map(|v| v as i32),
            invigilator: row.invigilator.clone(),
            start: row.start,
            end: row.end,
            status: row.status as i32,
        }
    }
}

// ---------------------------------------------------------------------------
// 18. Grades
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct GradeRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Text)]
    pub exam: String,
    #[diesel(sql_type = Integer)]
    pub student: i32,
    #[diesel(sql_type = SmallInt)]
    pub subject: i16,
    #[diesel(sql_type = Nullable<SmallInt>)]
    pub paper: Option<i16>,
    #[diesel(sql_type = Float)]
    pub score: f32,
    #[diesel(sql_type = Integer)]
    pub total: i32,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl GradeRow {
    pub fn row_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            self.school,
            self.exam,
            self.student,
            self.subject,
            self.paper.map(|v| v.to_string()).unwrap_or_default()
        )
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&GradeRow> for GradeInsert {
    fn from(row: &GradeRow) -> Self {
        GradeInsert {
            school: row.school.clone(),
            exam: row.exam.clone(),
            student: row.student,
            subject: row.subject as i32,
            paper: row.paper.map(|v| v as i32),
            score: row.score,
            total: row.total,
        }
    }
}

// ---------------------------------------------------------------------------
// 19. Fees
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct FeeRow {
    #[diesel(sql_type = Text)]
    pub id: String,
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Integer)]
    pub year: i32,
    #[diesel(sql_type = SmallInt)]
    pub term: i16,
    #[diesel(sql_type = SmallInt)]
    pub grade: i16,
    #[diesel(sql_type = Text)]
    pub title: String,
    #[diesel(sql_type = Text)]
    pub description: String,
    #[diesel(sql_type = Float)]
    pub amount: f32,
    #[diesel(sql_type = Bool)]
    pub mandatory: bool,
    #[diesel(sql_type = BigInt)]
    pub due: i64,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl FeeRow {
    pub fn row_key(&self) -> String {
        self.id.clone()
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&FeeRow> for FeeInsert {
    fn from(row: &FeeRow) -> Self {
        FeeInsert {
            id: row.id.clone(),
            school: row.school.clone(),
            year: row.year,
            term: row.term as i32,
            grade: row.grade as i32,
            title: row.title.clone(),
            description: row.description.clone(),
            amount: row.amount,
            mandatory: row.mandatory,
            due: row.due,
        }
    }
}

// ---------------------------------------------------------------------------
// 20. Invoices
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct InvoiceRow {
    #[diesel(sql_type = Text)]
    pub id: String,
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Integer)]
    pub year: i32,
    #[diesel(sql_type = SmallInt)]
    pub term: i16,
    #[diesel(sql_type = Nullable<Text>)]
    pub fee: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub description: Option<String>,
    #[diesel(sql_type = Integer)]
    pub student: i32,
    #[diesel(sql_type = Float)]
    pub amount: f32,
    #[diesel(sql_type = SmallInt)]
    pub status: i16,
    #[diesel(sql_type = Nullable<BigInt>)]
    pub due: Option<i64>,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl InvoiceRow {
    pub fn row_key(&self) -> String {
        self.id.clone()
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&InvoiceRow> for InvoiceInsert {
    fn from(row: &InvoiceRow) -> Self {
        InvoiceInsert {
            id: row.id.clone(),
            school: row.school.clone(),
            year: row.year,
            term: row.term as i32,
            fee: row.fee.clone(),
            description: row.description.clone(),
            student: row.student,
            amount: row.amount,
            status: row.status as i32,
            due: row.due,
        }
    }
}

// ---------------------------------------------------------------------------
// 21. Payments
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct PaymentRow {
    #[diesel(sql_type = Text)]
    pub id: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub invoice: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub school: Option<String>,
    #[diesel(sql_type = Nullable<Integer>)]
    pub student: Option<i32>,
    #[diesel(sql_type = Float)]
    pub amount: f32,
    #[diesel(sql_type = SmallInt)]
    pub method: i16,
    #[diesel(sql_type = Nullable<Text>)]
    pub reference: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub recorder: Option<String>,
    #[diesel(sql_type = Nullable<Integer>)]
    pub date: Option<i32>,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl PaymentRow {
    pub fn row_key(&self) -> String {
        self.id.clone()
    }
    pub fn school_id(&self) -> Option<&str> {
        self.school.as_deref()
    }
}

impl From<&PaymentRow> for PaymentInsert {
    fn from(row: &PaymentRow) -> Self {
        PaymentInsert {
            id: row.id.clone(),
            invoice: row.invoice.clone(),
            school: row.school.clone(),
            student: row.student,
            amount: row.amount,
            method: row.method as i32,
            reference: row.reference.clone(),
            recorder: row.recorder.clone(),
            date: row.date,
        }
    }
}

// ---------------------------------------------------------------------------
// 22. Announcements
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct AnnouncementRow {
    #[diesel(sql_type = Text)]
    pub id: String,
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Text)]
    pub title: String,
    #[diesel(sql_type = Text)]
    pub content: String,
    #[diesel(sql_type = Nullable<SmallInt>)]
    pub grade: Option<i16>,
    #[diesel(sql_type = Nullable<SmallInt>)]
    pub stream: Option<i16>,
    #[diesel(sql_type = Integer)]
    pub audience: i32,
    #[diesel(sql_type = Nullable<Text>)]
    pub author: Option<String>,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl AnnouncementRow {
    pub fn row_key(&self) -> String {
        self.id.clone()
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&AnnouncementRow> for AnnouncementInsert {
    fn from(row: &AnnouncementRow) -> Self {
        AnnouncementInsert {
            id: row.id.clone(),
            school: row.school.clone(),
            title: row.title.clone(),
            content: row.content.clone(),
            grade: row.grade.map(|v| v as i32),
            stream: row.stream.map(|v| v as i32),
            audience: row.audience,
            author: row.author.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// 23. Mastery
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct MasteryRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Integer)]
    pub student: i32,
    #[diesel(sql_type = SmallInt)]
    pub grade: i16,
    #[diesel(sql_type = SmallInt)]
    pub subject: i16,
    #[diesel(sql_type = SmallInt)]
    pub topic: i16,
    #[diesel(sql_type = Float)]
    pub score: f32,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl MasteryRow {
    pub fn row_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            self.school, self.student, self.grade, self.subject, self.topic
        )
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&MasteryRow> for MasteryInsert {
    fn from(row: &MasteryRow) -> Self {
        MasteryInsert {
            school: row.school.clone(),
            student: row.student,
            grade: row.grade as i32,
            subject: row.subject as i32,
            topic: row.topic as i32,
            score: row.score,
        }
    }
}

// ---------------------------------------------------------------------------
// 24. AiUsage
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct AiUsageRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Integer)]
    pub student: i32,
    #[diesel(sql_type = Integer)]
    pub year: i32,
    #[diesel(sql_type = SmallInt)]
    pub term: i16,
    #[diesel(sql_type = Integer)]
    pub allocated: i32,
    #[diesel(sql_type = Integer)]
    pub used: i32,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl AiUsageRow {
    pub fn row_key(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.school, self.student, self.year, self.term
        )
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&AiUsageRow> for AiUsageInsert {
    fn from(row: &AiUsageRow) -> Self {
        AiUsageInsert {
            school: row.school.clone(),
            student: row.student,
            year: row.year,
            term: row.term as i32,
            allocated: row.allocated,
            used: row.used,
        }
    }
}

// ---------------------------------------------------------------------------
// 25. Settings
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct SettingsRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Text)]
    pub data: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub mpesa: Option<String>,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl SettingsRow {
    pub fn row_key(&self) -> String {
        self.school.clone()
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&SettingsRow> for SettingsInsert {
    fn from(row: &SettingsRow) -> Self {
        SettingsInsert {
            school: row.school.clone(),
            data: row.data.clone(),
            mpesa: row.mpesa.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// 26. Roles
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct RoleRow {
    #[diesel(sql_type = Text)]
    pub id: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub school: Option<String>,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub description: Option<String>,
    #[diesel(sql_type = Binary)]
    pub permissions: Vec<u8>,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl RoleRow {
    pub fn row_key(&self) -> String {
        self.id.clone()
    }
    pub fn school_id(&self) -> Option<&str> {
        self.school.as_deref()
    }
}

impl From<&RoleRow> for RoleInsert {
    fn from(row: &RoleRow) -> Self {
        RoleInsert {
            id: row.id.clone(),
            school: row.school.clone(),
            name: row.name.clone(),
            description: row.description.clone(),
            permissions: row.permissions.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// 27. Scopes
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct ScopeRow {
    #[diesel(sql_type = Nullable<Text>)]
    pub school: Option<String>,
    #[diesel(sql_type = Text)]
    pub user: String,
    #[diesel(sql_type = Text)]
    pub role: String,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
}

impl ScopeRow {
    pub fn row_key(&self) -> String {
        format!(
            "{}|{}|{}",
            self.school.as_deref().unwrap_or(""),
            self.user,
            self.role
        )
    }
    pub fn school_id(&self) -> Option<&str> {
        self.school.as_deref()
    }
}

impl From<&ScopeRow> for ScopeInsert {
    fn from(row: &ScopeRow) -> Self {
        ScopeInsert {
            school: row.school.clone(),
            user: row.user.clone(),
            role: row.role.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// 28. Plans
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct PlanRow {
    #[diesel(sql_type = Text)]
    pub id: String,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub description: Option<String>,
    #[diesel(sql_type = Float)]
    pub amount: f32,
    #[diesel(sql_type = Integer)]
    pub levels: i32,
    #[diesel(sql_type = SmallInt)]
    pub status: i16,
    #[diesel(sql_type = Nullable<Text>)]
    pub features: Option<String>,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl PlanRow {
    pub fn row_key(&self) -> String {
        self.id.clone()
    }
    pub fn school_id(&self) -> Option<&str> {
        None
    }
}

impl From<&PlanRow> for PlanInsert {
    fn from(row: &PlanRow) -> Self {
        PlanInsert {
            id: row.id.clone(),
            name: row.name.clone(),
            description: row.description.clone(),
            amount: row.amount,
            levels: row.levels,
            status: row.status as i32,
            features: row.features.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// 29. Subscriptions
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct SubscriptionRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Text)]
    pub plan: String,
    #[diesel(sql_type = Integer)]
    pub year: i32,
    #[diesel(sql_type = SmallInt)]
    pub term: i16,
    #[diesel(sql_type = Integer)]
    pub student: i32,
    #[diesel(sql_type = Nullable<Text>)]
    pub invoice: Option<String>,
    #[diesel(sql_type = Float)]
    pub discount: f32,
    #[diesel(sql_type = SmallInt)]
    pub status: i16,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl SubscriptionRow {
    pub fn row_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            self.school, self.plan, self.year, self.term, self.student
        )
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&SubscriptionRow> for SubscriptionInsert {
    fn from(row: &SubscriptionRow) -> Self {
        SubscriptionInsert {
            school: row.school.clone(),
            plan: row.plan.clone(),
            year: row.year,
            term: row.term as i32,
            student: row.student,
            invoice: row.invoice.clone(),
            discount: row.discount,
            status: row.status as i32,
        }
    }
}

// ---------------------------------------------------------------------------
// 30. Discounts
// ---------------------------------------------------------------------------

#[derive(QueryableByName)]
pub struct DiscountRow {
    #[diesel(sql_type = Text)]
    pub school: String,
    #[diesel(sql_type = Text)]
    pub plan: String,
    #[diesel(sql_type = Integer)]
    pub year: i32,
    #[diesel(sql_type = SmallInt)]
    pub term: i16,
    #[diesel(sql_type = SmallInt)]
    pub grade: i16,
    #[diesel(sql_type = Float)]
    pub amount: f32,
    #[diesel(sql_type = SmallInt)]
    pub unit: i16,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
    #[diesel(sql_type = BigInt)]
    pub updated: i64,
}

impl DiscountRow {
    pub fn row_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            self.school, self.plan, self.year, self.term, self.grade
        )
    }
    pub fn school_id(&self) -> Option<&str> {
        Some(&self.school)
    }
}

impl From<&DiscountRow> for DiscountInsert {
    fn from(row: &DiscountRow) -> Self {
        DiscountInsert {
            school: row.school.clone(),
            plan: row.plan.clone(),
            year: row.year,
            term: row.term as i32,
            grade: row.grade as i32,
            amount: row.amount,
            unit: row.unit as i32,
        }
    }
}
