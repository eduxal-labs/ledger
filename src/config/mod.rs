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
use verifyer::Verifyer;

pub trait Config: Messenger<Recipient = Phone> + Verifyer {}
impl Config for Configuration {}

#[derive(Clone, Default)]
pub struct Configuration {
    verifyer: verifications::Verifications,
    messenger: whatsapp::Whatsapp,
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
