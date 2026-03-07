use crate::types::command::{Command, Receiver, Response, Sender};
use crate::types::{
    error::{Error, Result},
    id::Id,
    phone::Phone,
    verification::{Code, Purpose, Verification},
};
use ahash::HashMap;
use chrono::Utc;
use crossbeam::channel::bounded;
use std::collections::BTreeMap;
use std::collections::hash_map::Entry;
use std::time::Duration;

type Data = HashMap<Id, Verification>;
type Index = HashMap<Phone, Id>;
type Expiries = BTreeMap<i64, Vec<Id>>;

const LIMIT: Duration = Duration::from_secs(90);

#[derive(Clone)]
pub struct Verifications {
    sender: Sender,
}

#[derive(Default)]
struct Processor {
    data: Data,
    index: Index,
    expiries: Expiries,
}

impl Default for Verifications {
    fn default() -> Self {
        let (sender, receiver) = bounded(200);
        Self::start(receiver);
        Self { sender }
    }
}

impl Verifications {
    pub fn send(&self, command: Command) -> Result<()> {
        Ok(self.sender.try_send(command)?)
    }

    fn start(receiver: Receiver) {
        std::thread::spawn(move || Self::processor(receiver));
    }

    pub fn processor(receiver: Receiver) {
        let mut processor = Processor::default();

        loop {
            if let Ok(command) = receiver.try_recv() {
                match command {
                    Command::Request(phone, user, purpose, responder) => {
                        #[allow(unused)]
                        responder.send(Response::Requested(
                            processor.generate(phone, user, purpose),
                        ));
                    }
                    Command::Verify(id, code, user, purpose, responder) => {
                        #[allow(unused)]
                        responder.send(Response::Verified(
                            processor.verify(id, code, user, purpose),
                        ));
                    }
                    Command::Delete(id) => processor.delete(id),
                };
            }
            processor.clean();
        }
    }
}

impl Processor {
    pub fn generate(
        &mut self,
        phone: Phone,
        user: Option<Id>,
        purpose: Purpose,
    ) -> Result<Verification> {
        let data = &mut self.data;
        let index = &mut self.index;
        let expiries = &mut self.expiries;

        if let Some(id) = index.get(&phone)
            && let Some(verification) = data.get(id)
        {
            let limit = verification.created + LIMIT;
            let current = Utc::now();
            if current >= limit {
                return Err(Error::SlowDown);
            }
        }
        let verification = Verification::new(phone, user, purpose);
        let id = verification.id;
        let expiry =
            (verification.created + Duration::from_secs(verification.ttl as u64)).timestamp();
        data.insert(id, verification);
        index.insert(phone, id);
        expiries.entry(expiry).or_default().push(id);
        Ok(verification)
    }

    pub fn verify(
        &mut self,
        id: Id,
        code: Code,
        user: Option<Id>,
        purpose: Purpose,
    ) -> Result<Verification> {
        if let Entry::Occupied(entry) = self.data.entry(id) {
            let verification = entry.get();
            let expiry = verification.created + Duration::from_secs(verification.ttl as u64);
            let current = Utc::now();
            if current >= expiry {
                return Err(Error::InvalidVerificationCode);
            }
            if verification.purpose != purpose {
                return Err(Error::InvalidVerificationCode);
            }
            if let Some(user) = user
                && let Some(id) = verification.user
                && id != user
            {
                return Err(Error::Unauthorized);
            }
            if code != verification.code {
                return Err(Error::InvalidVerificationCode);
            }
            let verification = entry.remove();
            self.index.remove(&verification.phone);
            self.expiries
                .entry(verification.created.timestamp())
                .or_default()
                .retain(|&i| i != id);
            return Ok(verification);
        }
        Err(Error::InvalidVerificationCode)
    }

    pub fn delete(&mut self, id: Id) {
        if let Some(verification) = self.data.remove(&id) {
            // remove the phone index and leave the expiries to be handled by the cleaner.
            self.index.remove(&verification.phone);
        }
    }

    pub fn clean(&mut self) {
        let expiries = &mut self.expiries;
        let index = &mut self.index;
        let data = &mut self.data;
        let expired = Utc::now().timestamp();

        //split the btree to get all the itmes that have expired and keep the rest.
        let unexpired = expiries.split_off(&expired);
        let expired = std::mem::take(expiries);
        *expiries = unexpired;

        for (_, ids) in expired {
            for id in ids {
                if let Some(Verification { phone, .. }) = data.remove(&id) {
                    index.remove(&phone);
                }
            }
        }
    }
}
