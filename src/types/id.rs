use crate::types::error::Error;
use bson::oid::ObjectId;
use diesel::deserialize::FromSqlRow;
use diesel::serialize::IsNull;
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    expression::AsExpression,
    serialize::{self, Output, ToSql},
};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Hash, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
pub struct Id([u8; 12]);

impl Id {
    pub fn system() -> Self {
        Self([0u8; 12])
    }
}

impl Default for Id {
    fn default() -> Self {
        Id(ObjectId::new().bytes())
    }
}

impl FromStr for Id {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = ObjectId::parse_str(s)
            .map_err(|_| Error::InvalidId)?
            .bytes();
        Ok(Self(bytes))
    }
}

impl Debug for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", ObjectId::from_bytes(self.0).to_hex())
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", ObjectId::from_bytes(self.0).to_hex())
    }
}

impl From<Id> for String {
    fn from(value: Id) -> Self {
        value.to_string()
    }
}

impl TryFrom<String> for Id {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<[u8; 12]> for Id {
    fn from(bytes: [u8; 12]) -> Self {
        Id(bytes)
    }
}

impl TryFrom<&[u8]> for Id {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 12 {
            return Err(Error::InvalidId);
        }
        let mut id = [0u8; 12];
        id.copy_from_slice(bytes);
        Ok(Id(id))
    }
}

impl TryFrom<Vec<u8>> for Id {
    type Error = Error;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let slice = bytes.as_slice();
        Self::try_from(slice)
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&ObjectId::from_bytes(self.0).to_hex())
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex = <&str>::deserialize(deserializer)?;
        let bytes = ObjectId::parse_str(hex)
            .map_err(serde::de::Error::custom)?
            .bytes();
        Ok(Self(bytes))
    }
}

impl ToSql<Text, Sqlite> for Id {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

impl<DB> FromSql<Text, DB> for Id
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    /// Deserializes an `Id` from a SQL database.
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let string = String::from_sql(bytes)?;
        Ok(string.parse()?)
    }
}
