use crate::types::{error::Error, id::Id, phone::Phone};
use chrono::{DateTime, Utc};
use rand::{RngExt, rng};
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Verification {
    pub id: Id,
    pub phone: Phone,
    pub user: Option<Id>,
    pub code: Code,
    pub purpose: Purpose,
    pub ttl: i32,
    pub created: DateTime<Utc>,
}

const LENGTH: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Code([u8; LENGTH]);

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Purpose {
    #[default]
    Verify = 0,
    ChangePhone = 1,
    Delete = 2,
}

impl From<Purpose> for i32 {
    fn from(purpose: Purpose) -> Self {
        use crate::proto::types::verification::Purpose as Proto;
        match purpose {
            Purpose::Verify => Proto::Verify as i32,
            Purpose::ChangePhone => Proto::ChangePhone as i32,
            Purpose::Delete => Proto::Delete as i32,
        }
    }
}

impl Verification {
    //TTL in minutes.
    const TTL: i32 = 15 * 60;
    pub fn new(phone: Phone, user: Option<Id>, purpose: Purpose) -> Self {
        let id = Id::default();
        let code = Code::default();
        let ttl = Self::TTL;
        let created = Utc::now();

        Self {
            id,
            phone,
            user,
            code,
            purpose,
            ttl,
            created,
        }
    }
}

impl Default for Code {
    fn default() -> Self {
        let digits: [u8; 6] = std::array::from_fn(|_| rng().random_range(b'0'..=b'9'));
        Self(digits)
    }
}

impl AsRef<str> for Code {
    fn as_ref(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
}

impl FromStr for Code {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let digits = value.as_bytes();
        if digits.len() != LENGTH {
            return Err(Error::InvalidVerificationCode);
        }
        let mut bytes = [0u8; LENGTH];
        bytes.copy_from_slice(digits);
        Ok(Self(bytes))
    }
}

impl From<Verification> for crate::proto::types::verification::Verification {
    fn from(verification: Verification) -> Self {
        let id = verification.id.into();
        let phone = verification.phone.into();
        let user = verification.user.map(|id| id.into());
        let purpose = verification.purpose.into();
        Self {
            id,
            phone,
            user,
            purpose,
        }
    }
}

unsafe impl Send for Code {}
unsafe impl Sync for Code {}
unsafe impl Send for Verification {}
unsafe impl Sync for Verification {}
