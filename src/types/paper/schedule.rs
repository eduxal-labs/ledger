use crate::db::schema::paper_schedules;
use crate::types::id::Id;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::SmallInt;
use diesel::sqlite::Sqlite;
use diesel::{Insertable, Queryable, QueryableByName, Selectable};

// ── GenerationStatus ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum GenerationStatus {
    #[default]
    Pending = 0,
    Generating = 1,
    Generated = 2,
    Failed = 3,
}

impl TryFrom<i16> for GenerationStatus {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Pending),
            1 => Ok(Self::Generating),
            2 => Ok(Self::Generated),
            3 => Ok(Self::Failed),
            _ => Err(crate::types::error::Error::PaperScheduleNotFound),
        }
    }
}

impl From<GenerationStatus> for i16 {
    fn from(v: GenerationStatus) -> i16 {
        v as i16
    }
}

impl ToSql<SmallInt, Sqlite> for GenerationStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self as i32);
        Ok(IsNull::No)
    }
}

impl FromSql<SmallInt, Sqlite> for GenerationStatus {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

// ── PaperSchedule ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = paper_schedules)]
pub struct PaperSchedule {
    pub id: Id,
    pub event: String,
    pub subject: i32,
    pub grade: i16,
    pub stream: Option<i16>,
    pub date: i32,
    pub start_time: i32,
    pub end_time: i32,
    pub duration_minutes: i16,
    pub invigilator: Option<String>,
    pub paper: Option<String>,
    pub generation_status: GenerationStatus,
    pub reveal_at: i64,
    pub generate_at: i64,
    pub created: i64,
}

// ── PaperScheduleUpdate ─────────────────────────────────────────────────────

#[derive(Debug, Default, diesel::AsChangeset)]
#[diesel(table_name = paper_schedules)]
pub struct PaperScheduleUpdate {
    pub date: Option<i32>,
    pub start_time: Option<i32>,
    pub end_time: Option<i32>,
    pub duration_minutes: Option<i16>,
    pub invigilator: Option<Option<String>>,
    pub paper: Option<Option<String>>,
    pub generation_status: Option<GenerationStatus>,
    pub reveal_at: Option<i64>,
    pub generate_at: Option<i64>,
}
