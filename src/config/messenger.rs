use crate::types::error::Result;
use std::future::Future;

pub trait Messenger {
    type Recipient: AsRef<str>;
    fn send_code(
        &self,
        recipient: &Self::Recipient,
        code: &str,
    ) -> impl Future<Output = Result<()>> + Send;
}
