use crate::config::Config;
use crate::db::database::CONN as conn;
use crate::db::database::traits::Database;
use crate::proto::services::authentication::{
    Authenticated, Authentication, AuthenticationServer, Verified,
};
use crate::types::{
    error::{Error, Result},
    id::Id,
    phone::Phone,
    token::Token,
    user::{self, Status, User},
    verification::{Code, Purpose, Verification},
};
use std::sync::Arc;

pub struct Authenticator<C> {
    config: Arc<C>,
}

impl<C: Config + Send + Sync + 'static> Authentication for Authenticator<C> {
    type Config = Arc<C>;
    fn new(config: Self::Config) -> AuthenticationServer<Self> {
        AuthenticationServer::new(Self { config })
    }

    async fn login(&self, phone: Phone) -> Result<Verification> {
        let user = match conn.find::<_, User>(phone)? {
            Some(user) => {
                if user.status == Status::Suspended {
                    return Err(Error::Forbidden);
                }
                Some(user.id)
            }
            None => None,
        };
        let purpose = Purpose::Verify;
        let verification = self.config.create(phone, user, purpose).await?;
        let code = verification.code.as_ref();
        if let Err(err) = self.config.send_code(&phone, code).await {
            self.config.delete(verification.id)?;
            return Err(err)?;
        };
        Ok(verification)
    }

    async fn verify(&self, id: Id, code: Code) -> Result<Verified> {
        let purpose = Purpose::Verify;
        let Verification { phone, .. } = self.config.verify(id, code, None, purpose).await?;
        let user = conn.find(phone)?;
        let verified = match user {
            Some(mut user) => {
                if user.status == Status::Suspended {
                    return Err(Error::Forbidden);
                }
                let status = Some(Status::Active);
                let updated = Some(chrono::Utc::now().timestamp());
                let update = user::Update {
                    status,
                    updated,
                    ..Default::default()
                };
                user = conn.update(user.id, update)?;
                Verified::authenticated(user)
            }
            None => Verified::registered(phone),
        }?;
        Ok(verified)
    }

    fn setup(&self, token: Token, name: String) -> Result<Authenticated> {
        token.validate_setup()?;
        let phone = token.phone;
        let user = conn.find(phone)?;
        if let Some(mut user) = user {
            if user.status == Status::Suspended {
                return Err(Error::Forbidden);
            }
            let status = Some(Status::Active);
            let name = Some(name);
            let updated = Some(chrono::Utc::now().timestamp());
            let update = user::Update {
                status,
                name,
                updated,
                ..Default::default()
            };
            user = conn.update(user.id, update)?;
            return Ok(Authenticated::new(user)?);
        }
        let user = User::new(phone, name);
        let user = conn.create(user)?;
        Ok(Authenticated::new(user)?)
    }

    fn refresh(&self, token: Token) -> Result<Authenticated> {
        token.validate_refresh()?;
        let id = token.user;
        let user = conn.find(id)?.ok_or(Error::UserNotFound)?;
        Ok(Authenticated::new(user)?)
    }

    async fn change_phone(&self, token: Token, phone: Phone) -> Result<Verification> {
        let user = conn.find(token.user)?.ok_or(Error::UserNotFound)?;
        if user.phone == phone {
            return Err(Error::NothingToUpdate);
        }
        let purpose = Purpose::ChangePhone;
        let user = Some(user.id);
        let verification = self.config.create(phone, user, purpose).await?;
        let code = verification.code.as_ref();
        if let Err(err) = self.config.send_code(&phone, code).await {
            self.config.delete(verification.id)?;
            return Err(err)?;
        };
        Ok(verification)
    }

    async fn confirm_change_phone(
        &self,
        token: Token,
        id: Id,
        code: Code,
    ) -> Result<Authenticated> {
        let user = Some(token.user);
        let purpose = Purpose::ChangePhone;
        let Verification { phone, .. } = self.config.verify(id, code, user, purpose).await?;
        let phone = Some(phone);
        let record = user::Update {
            phone,
            ..Default::default()
        };
        let user = conn.update(token.user, record)?;
        let authenticated = Authenticated::new(user)?;
        Ok(authenticated)
    }
}
