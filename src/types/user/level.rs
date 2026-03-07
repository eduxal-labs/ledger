use crate::types::error::Error;
use diesel::sql_types::SmallInt;
use diesel::{AsExpression, FromSqlRow};

#[derive(AsExpression, FromSqlRow)]
#[diesel(sql_type = SmallInt)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[repr(u8)]
pub enum Level {
    #[default]
    Normal = 0,
    System = 1,
    Super = 2,
}

impl TryFrom<i32> for Level {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Level::Normal),
            1 => Ok(Level::System),
            2 => Ok(Level::Super),
            _ => Err(Error::InvalidLevel),
        }
    }
}

impl From<Level> for i32 {
    fn from(level: Level) -> Self {
        match level {
            Level::Normal => 0,
            Level::System => 1,
            Level::Super => 2,
        }
    }
}

impl<DB> diesel::serialize::ToSql<diesel::sql_types::SmallInt, DB> for Level
where
    DB: diesel::backend::Backend,
    i16: diesel::serialize::ToSql<diesel::sql_types::SmallInt, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        match self {
            Level::Normal => 0.to_sql(out),
            Level::System => 1.to_sql(out),
            Level::Super => 2.to_sql(out),
        }
    }
}

impl<DB> diesel::deserialize::FromSql<diesel::sql_types::SmallInt, DB> for Level
where
    DB: diesel::backend::Backend,
    i16: diesel::deserialize::FromSql<diesel::sql_types::SmallInt, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let level = i16::from_sql(bytes)?;
        match level {
            0 => Ok(Level::Normal),
            1 => Ok(Level::System),
            2 => Ok(Level::Super),
            _ => Err("invalid level".into()),
        }
    }
}
