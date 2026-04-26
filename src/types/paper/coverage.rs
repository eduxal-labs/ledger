use crate::db::schema::{exam_coverage, paper_topics, taught_topics};
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::SmallInt;
use diesel::sqlite::Sqlite;
use diesel::{Insertable, Queryable, QueryableByName, Selectable};

// ── TaughtStatus ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum TaughtStatus {
    #[default]
    NotStarted = 0,
    InProgress = 1,
    Completed = 2,
}

impl TryFrom<i16> for TaughtStatus {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::NotStarted),
            1 => Ok(Self::InProgress),
            2 => Ok(Self::Completed),
            _ => Err(crate::types::error::Error::NotFound),
        }
    }
}

impl From<TaughtStatus> for i16 {
    fn from(v: TaughtStatus) -> i16 {
        v as i16
    }
}

impl ToSql<SmallInt, Sqlite> for TaughtStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self as i32);
        Ok(IsNull::No)
    }
}

impl FromSql<SmallInt, Sqlite> for TaughtStatus {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

// ── TaughtTopic ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = taught_topics)]
pub struct TaughtTopic {
    pub school: String,
    pub subject: i32,
    pub grade: i16,
    pub stream: Option<i16>,
    pub topic: i32,
    pub taught_by: String,
    pub status: TaughtStatus,
    pub taught_date: Option<i32>,
    pub updated: i64,
}

// ── TaughtTopicUpdate ────────────────────────────────────────────────────────

#[derive(Debug, Default, diesel::AsChangeset)]
#[diesel(table_name = taught_topics)]
pub struct TaughtTopicUpdate {
    pub taught_by: Option<String>,
    pub status: Option<TaughtStatus>,
    pub taught_date: Option<Option<i32>>,
    pub updated: Option<i64>,
}

// ── ExamCoverage ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = exam_coverage)]
pub struct ExamCoverage {
    pub schedule: String,
    pub topic: i32,
    pub confirmed_by: String,
    pub confirmed_at: i64,
}

// ── PaperTopic ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = paper_topics)]
pub struct PaperTopic {
    pub paper: String,
    pub topic: i32,
    pub weight: f32,
}
