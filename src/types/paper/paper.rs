use crate::db::schema::papers;
use crate::types::id::Id;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::SmallInt;
use diesel::sqlite::Sqlite;
use diesel::{Insertable, Queryable, QueryableByName, Selectable};

// ── PaperType ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum PaperType {
    #[default]
    Exam = 0,
    Cat = 1,
    Assessment = 2,
    Assignment = 3,
    Practical = 4,
    Adaptive = 5,
}

impl TryFrom<i16> for PaperType {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Exam),
            1 => Ok(Self::Cat),
            2 => Ok(Self::Assessment),
            3 => Ok(Self::Assignment),
            4 => Ok(Self::Practical),
            5 => Ok(Self::Adaptive),
            _ => Err(crate::types::error::Error::NotFound),
        }
    }
}

impl From<PaperType> for i16 {
    fn from(v: PaperType) -> i16 {
        v as i16
    }
}

impl ToSql<SmallInt, Sqlite> for PaperType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self as i32);
        Ok(IsNull::No)
    }
}

impl FromSql<SmallInt, Sqlite> for PaperType {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

// ── PaperStatus ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum PaperStatus {
    #[default]
    Draft = 0,
    QuestionsSet = 1,
    Finalized = 2,
    Revealed = 3,
    Active = 4,
    Completed = 5,
    Marked = 6,
}

impl TryFrom<i16> for PaperStatus {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Draft),
            1 => Ok(Self::QuestionsSet),
            2 => Ok(Self::Finalized),
            3 => Ok(Self::Revealed),
            4 => Ok(Self::Active),
            5 => Ok(Self::Completed),
            6 => Ok(Self::Marked),
            _ => Err(crate::types::error::Error::NotFound),
        }
    }
}

impl From<PaperStatus> for i16 {
    fn from(v: PaperStatus) -> i16 {
        v as i16
    }
}

impl ToSql<SmallInt, Sqlite> for PaperStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self as i32);
        Ok(IsNull::No)
    }
}

impl FromSql<SmallInt, Sqlite> for PaperStatus {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

// ── GenerationMode ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum GenerationMode {
    #[default]
    ClassUniform = 0,
    PerStudent = 1,
}

impl TryFrom<i16> for GenerationMode {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::ClassUniform),
            1 => Ok(Self::PerStudent),
            _ => Err(crate::types::error::Error::NotFound),
        }
    }
}

impl From<GenerationMode> for i16 {
    fn from(v: GenerationMode) -> i16 {
        v as i16
    }
}

impl ToSql<SmallInt, Sqlite> for GenerationMode {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self as i32);
        Ok(IsNull::No)
    }
}

impl FromSql<SmallInt, Sqlite> for GenerationMode {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

// ── Paper ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = papers)]
pub struct Paper {
    pub id: Id,
    pub school: String,
    pub event: Option<String>,
    pub subject: i32,
    pub grade: i16,
    pub stream: Option<i16>,
    pub type_: PaperType,
    pub teacher: String,
    pub name: String,
    pub total_marks: i16,
    pub duration_minutes: i16,
    pub date: i32,
    pub status: PaperStatus,
    pub pdf_key: Option<String>,
    pub ms_key: Option<String>,
    pub generation_mode: GenerationMode,
    pub instructions: Option<String>,
    pub created: i64,
    pub updated: i64,
}

// ── PaperUpdate ─────────────────────────────────────────────────────────────

#[derive(Debug, Default, diesel::AsChangeset)]
#[diesel(table_name = papers)]
pub struct PaperUpdate {
    pub name: Option<String>,
    pub event: Option<Option<String>>,
    pub grade: Option<i16>,
    pub stream: Option<Option<i16>>,
    pub type_: Option<PaperType>,
    pub total_marks: Option<i16>,
    pub duration_minutes: Option<i16>,
    pub date: Option<i32>,
    pub status: Option<PaperStatus>,
    pub pdf_key: Option<Option<String>>,
    pub ms_key: Option<Option<String>>,
    pub generation_mode: Option<GenerationMode>,
    pub instructions: Option<Option<String>>,
    pub updated: Option<i64>,
}
