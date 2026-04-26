use crate::db::schema::events;
use crate::types::id::Id;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::SmallInt;
use diesel::sqlite::Sqlite;
use diesel::{Insertable, Queryable, QueryableByName, Selectable};

// ── EventType ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum EventType {
    #[default]
    Exam = 0,
    Mock = 1,
    HolidayRevision = 2,
}

impl TryFrom<i16> for EventType {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Exam),
            1 => Ok(Self::Mock),
            2 => Ok(Self::HolidayRevision),
            _ => Err(crate::types::error::Error::NotFound),
        }
    }
}
impl From<EventType> for i16 {
    fn from(v: EventType) -> i16 {
        v as i16
    }
}
impl ToSql<SmallInt, Sqlite> for EventType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self as i32);
        Ok(IsNull::No)
    }
}
impl FromSql<SmallInt, Sqlite> for EventType {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

// ── EventStatus ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum EventStatus {
    #[default]
    Draft = 0,
    Active = 1,
    Completed = 2,
    Cancelled = 3,
}

impl TryFrom<i16> for EventStatus {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Draft),
            1 => Ok(Self::Active),
            2 => Ok(Self::Completed),
            3 => Ok(Self::Cancelled),
            _ => Err(crate::types::error::Error::NotFound),
        }
    }
}
impl From<EventStatus> for i16 {
    fn from(v: EventStatus) -> i16 {
        v as i16
    }
}
impl ToSql<SmallInt, Sqlite> for EventStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self as i32);
        Ok(IsNull::No)
    }
}
impl FromSql<SmallInt, Sqlite> for EventStatus {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

// ── Event ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = events)]
pub struct Event {
    pub id: Id,
    pub school: String,
    pub name: String,
    pub type_: EventType,
    pub term: i16,
    pub year: i32,
    pub start_date: i32,
    pub end_date: i32,
    pub status: EventStatus,
    pub created: i64,
    pub updated: i64,
}

// ── EventUpdate ─────────────────────────────────────────────────────────────

#[derive(Debug, Default, diesel::AsChangeset)]
#[diesel(table_name = events)]
pub struct EventUpdate {
    pub name: Option<String>,
    pub type_: Option<EventType>,
    pub term: Option<i16>,
    pub year: Option<i32>,
    pub start_date: Option<i32>,
    pub end_date: Option<i32>,
    pub status: Option<EventStatus>,
    pub updated: Option<i64>,
}
