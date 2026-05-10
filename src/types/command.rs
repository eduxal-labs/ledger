use crate::types::{
    error::{Error, Result},
    id::Id,
    phone::Phone,
    verification::{Code, Purpose, Verification},
};
use crossbeam::channel;
use tokio::sync::oneshot;

pub type Responder = oneshot::Sender<Response>;
pub type Sender = channel::Sender<Command>;
pub type Receiver = channel::Receiver<Command>;

pub enum Command {
    Request(Phone, Option<Id>, Purpose, Responder),
    Verify(Id, Code, Option<Id>, Purpose, Responder),
    Delete(Id),
}

pub enum Response {
    Requested(Result<Verification>),
    Verified(Result<Verification>),
}

impl Response {
    pub fn requested(self) -> Result<Verification> {
        match self {
            Response::Requested(result) => result,
            _ => Err(Error::Internal(
                "unexpected auth response variant: expected Requested".into(),
            )),
        }
    }

    pub fn verified(self) -> Result<Verification> {
        match self {
            Response::Verified(result) => result,
            _ => Err(Error::Internal(
                "unexpected auth response variant: expected Verified".into(),
            )),
        }
    }
}
