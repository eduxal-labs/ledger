use super::verifications::Verifications;
use crate::types::command::Command;
use crate::types::error::{Error, Result};
use crate::types::id::Id;
use crate::types::phone::Phone;
use crate::types::verification::{Code, Purpose, Verification};
use std::future::Future;
use tokio::sync::oneshot;

pub trait Verifyer {
    fn create(
        &self,
        phone: Phone,
        user: Option<Id>,
        purpose: Purpose,
    ) -> impl Future<Output = Result<Verification>> + Send;
    fn verify(
        &self,
        id: Id,
        code: Code,
        user: Option<Id>,
        purpose: Purpose,
    ) -> impl Future<Output = Result<Verification>> + Send;
    fn delete(&self, id: Id) -> Result<()>;
}

impl Verifyer for Verifications {
    async fn create(
        &self,
        phone: Phone,
        user: Option<Id>,
        purpose: Purpose,
    ) -> Result<Verification> {
        let (sender, receiver) = oneshot::channel();
        let command = Command::Request(phone, user, purpose, sender);
        self.send(command)?;
        receiver.await.map_err(Error::internal)?.requested()
    }

    async fn verify(
        &self,
        id: Id,
        code: Code,
        user: Option<Id>,
        purpose: Purpose,
    ) -> Result<Verification> {
        let (sender, receiver) = oneshot::channel();
        let command = Command::Verify(id, code, user, purpose, sender);
        self.send(command)?;
        receiver.await.map_err(Error::internal)?.verified()
    }

    fn delete(&self, id: Id) -> Result<()> {
        let command = Command::Delete(id);
        self.send(command)
    }
}
