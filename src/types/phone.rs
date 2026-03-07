use crate::types::error::Error;
use diesel::backend::Backend;
use diesel::deserialize::FromSqlRow;
use diesel::expression::AsExpression;
use diesel::sql_types::Text;
use diesel::{
    deserialize::{self, FromSql},
    serialize::{self, Output, ToSql},
};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
pub struct Phone([u8; 10]);

impl Phone {
    pub fn new(phone: &str) -> Result<Self, Error> {
        phone.parse()
    }

    pub fn is_empty(&self) -> bool {
        self.0 == [0; 10]
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl From<Phone> for String {
    fn from(phone: Phone) -> Self {
        String::from(phone.as_ref())
    }
}

impl TryFrom<String> for Phone {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl FromStr for Phone {
    type Err = Error;

    fn from_str(mut phone: &str) -> Result<Self, Self::Err> {
        if phone.len() != 13 && phone.len() != 12 && phone.len() != 10 && phone.len() != 9 {
            return Err(Error::InvalidPhone);
        }
        phone = phone.trim_start_matches("+");
        phone = phone.trim_start_matches("0");
        phone = phone.trim_start_matches("254");
        let bytes = phone.as_bytes();
        if bytes.len() != 9 {
            return Err(Error::InvalidPhone);
        }
        let mut phone = [0; 9];
        for (i, &byte) in bytes.iter().enumerate() {
            if !byte.is_ascii_digit() {
                return Err(Error::InvalidPhone);
            }
            phone[i] = byte;
        }
        let mut bytes = [0u8; 10];
        let prefix = b"0";
        bytes[..prefix.len()].copy_from_slice(prefix);
        bytes[prefix.len()..].copy_from_slice(&phone);
        Ok(Phone(bytes))
    }
}

///This converts the phone bytes into a utf-8 str without any checking
/// This is safe because the phone bytes are always valid utf-8
impl AsRef<str> for Phone {
    fn as_ref(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
}

impl Display for Phone {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl Debug for Phone {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_ref())
    }
}

impl Serialize for Phone {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_ref())
    }
}

impl<'de> Deserialize<'de> for Phone {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl<DB> ToSql<Text, DB> for Phone
where
    DB: Backend,
    str: ToSql<Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        let phone = self.as_ref();
        phone.to_sql(out)
    }
}

impl<DB> FromSql<Text, DB> for Phone
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let phone = String::from_sql(bytes)?;
        Ok(phone.parse()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phone_validation_and_representation() {
        let pass1 = "759762268";
        let pass2 = "+254759762268";
        let pass3 = "254759762268";
        let fail1 = "8790654397";
        let fail2 = "786690435t";

        let passed = Phone::new(pass1).unwrap();
        Phone::new(pass2).unwrap();
        Phone::new(pass3).unwrap();
        Phone::new(fail1).unwrap_err();
        Phone::new(fail2).unwrap_err();

        assert_eq!(passed.as_ref(), "0759762268");
    }
}
