use crate::types::error::Error;
use diesel::sql_types::SmallInt;
use diesel::{AsExpression, FromSqlRow};

#[derive(AsExpression, FromSqlRow)]
#[diesel(sql_type = SmallInt)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[repr(u8)]
pub enum Status {
    #[default]
    Invited = 0,
    Active = 1,
    Suspended = 2,
    Deleted = 3,
}

impl TryFrom<i32> for Status {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Status::Invited),
            1 => Ok(Status::Active),
            2 => Ok(Status::Suspended),
            3 => Ok(Status::Deleted),
            _ => Err(Error::InvalidStatus),
        }
    }
}

impl From<Status> for i32 {
    fn from(status: Status) -> Self {
        match status {
            Status::Invited => 0,
            Status::Active => 1,
            Status::Suspended => 2,
            Status::Deleted => 3,
        }
    }
}

impl<DB> diesel::serialize::ToSql<diesel::sql_types::SmallInt, DB> for Status
where
    DB: diesel::backend::Backend,
    i16: diesel::serialize::ToSql<diesel::sql_types::SmallInt, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        match self {
            Status::Invited => 0.to_sql(out),
            Status::Active => 1.to_sql(out),
            Status::Suspended => 2.to_sql(out),
            Status::Deleted => 3.to_sql(out),
        }
    }
}

impl<DB> diesel::deserialize::FromSql<SmallInt, DB> for Status
where
    DB: diesel::backend::Backend,
    i16: diesel::deserialize::FromSql<diesel::sql_types::SmallInt, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let status = i16::from_sql(bytes)?;
        match status {
            0 => Ok(Status::Invited),
            1 => Ok(Status::Active),
            2 => Ok(Status::Suspended),
            3 => Ok(Status::Deleted),
            _ => Err("invalid status".into()),
        }
    }
}
