use crate::types::error::{Error, Result};
use crate::types::id::Id;
use crate::types::phone::Phone;
use chrono::{DateTime, Duration, Utc};
use rand::RngExt;
use rusty_paseto::core::{Key, Local, Paseto, PasetoNonce, PasetoSymmetricKey, Payload, V4};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

macros::key!("PASETO_PASSWORD");

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Token {
    pub user: Id,
    pub phone: Phone,
    pub purpose: Purpose,
    pub created: DateTime<Utc>,
    pub expiry: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum Purpose {
    Access = 1,
    Refresh = 2,
    Setup = 3,
}

impl Token {
    fn new(user: Id, phone: Phone, purpose: Purpose) -> Self {
        let (created, expiry) = purpose.times();
        Self {
            user,
            phone,
            purpose,
            created,
            expiry,
        }
    }

    pub fn access(user: Id, phone: Phone) -> Self {
        Self::new(user, phone, Purpose::Access)
    }

    pub fn refresh(user: Id, phone: Phone) -> Self {
        Self::new(user, phone, Purpose::Refresh)
    }

    pub fn setup(user: Id, phone: Phone) -> Self {
        Self::new(user, phone, Purpose::Setup)
    }

    pub fn tokenize(&self) -> Result<String> {
        let key = Key::from(KEY);
        let key = PasetoSymmetricKey::<V4, Local>::from(key);
        let json = serde_json::to_string(&self).map_err(Error::internal)?;
        let payload = Payload::from(json.as_str());
        let mut nonce = [0u8; 32];
        rand::rng().fill(&mut nonce);
        let nonce = Key::from(nonce);
        let nonce = &PasetoNonce::from(&nonce);
        let token = Paseto::<V4, Local>::builder()
            .set_payload(payload)
            .try_encrypt(&key, nonce)
            .map_err(Error::internal)?;
        Ok(token)
    }

    pub fn validate_refresh(&self) -> Result<()> {
        if self.purpose != Purpose::Refresh {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }

    pub fn validate_setup(&self) -> Result<()> {
        if self.purpose != Purpose::Setup {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }
}

impl Purpose {
    const ACCESS_TTL: Duration = Duration::days(3);
    const REFRESH_TTL: Duration = Duration::days(30);
    const SETUP_TTL: Duration = Duration::minutes(60);

    pub fn times(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        let current = Utc::now();
        match self {
            Purpose::Access => (current, (current + Self::ACCESS_TTL)),
            Purpose::Refresh => (current, (current + Self::REFRESH_TTL)),
            Purpose::Setup => (current, (current + Self::SETUP_TTL)),
        }
    }
}

impl FromStr for Token {
    type Err = Error;

    fn from_str(token: &str) -> std::result::Result<Self, Self::Err> {
        let key = Key::from(KEY);
        let key = &PasetoSymmetricKey::from(key);
        let json = Paseto::<V4, Local>::try_decrypt(token, key, None, None)
            .map_err(Error::invalid_token)?;
        let token = serde_json::from_str::<Self>(&json).map_err(Error::invalid_token)?;
        if token.expiry.timestamp() < Utc::now().timestamp() {
            return Err(Error::Unauthorized);
        }
        Ok(token)
    }
}
