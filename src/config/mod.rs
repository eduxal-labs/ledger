mod messenger;
pub mod storage;
mod verifications;
mod verifyer;
mod whatsapp;

use crate::types::error::Result;
use crate::types::id::Id;
use crate::types::phone::Phone;
use crate::types::verification::{Code, Purpose, Verification};
use messenger::Messenger;
use std::sync::Arc;
use verifyer::Verifyer;

pub trait Config: Messenger<Recipient = Phone> + Verifyer {
    fn change_notifier(&self) -> &Arc<tokio::sync::Notify>;
}

impl Config for Configuration {
    fn change_notifier(&self) -> &Arc<tokio::sync::Notify> {
        &self.notifier
    }
}

#[derive(Clone)]
pub struct Configuration {
    verifyer: verifications::Verifications,
    messenger: whatsapp::Whatsapp,
    notifier: Arc<tokio::sync::Notify>,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            verifyer: Default::default(),
            messenger: Default::default(),
            notifier: Arc::new(tokio::sync::Notify::new()),
        }
    }
}

impl Messenger for Configuration {
    type Recipient = <whatsapp::Whatsapp as Messenger>::Recipient;
    async fn send_code(
        &self,
        recipient: &Self::Recipient,
        code: &str,
    ) -> crate::types::error::Result<()> {
        self.messenger.send_code(recipient, code).await
    }
}

impl Verifyer for Configuration {
    async fn create(
        &self,
        phone: Phone,
        user: Option<Id>,
        purpose: Purpose,
    ) -> Result<Verification> {
        self.verifyer.create(phone, user, purpose).await
    }

    async fn verify(
        &self,
        id: Id,
        code: Code,
        user: Option<Id>,
        purpose: Purpose,
    ) -> Result<Verification> {
        self.verifyer.verify(id, code, user, purpose).await
    }

    fn delete(&self, id: Id) -> Result<()> {
        self.verifyer.delete(id)
    }
}
