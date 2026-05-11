#![allow(dead_code)]

use crate::types::error::Result;
use crate::types::paginated::Paginated;
use crate::types::token::Token;
// (RefCell and LocalKey are no longer needed — Db uses Mutex instead)

pub trait Create<I, O = I> {
    fn create(&mut self, record: I) -> Result<O>;
}

pub trait Find<K, V> {
    fn find(&mut self, key: K) -> Result<Option<V>>;
}

pub trait Update<K, U, V = ()> {
    fn update(&mut self, key: K, record: U) -> Result<V>;
}

pub trait Load<I, O> {
    fn load(&mut self, input: I) -> Result<Vec<O>>;
}

// NOTE: Organisation and Permissions types do not exist yet.
// They will be created in Tasks 02-04. This trait will not compile
// until those tasks are complete. Import paths are pre-set to their
// final locations: crate::types::role::{Organisation, Permissions}.
pub trait Authorize {
    fn authorize(
        &mut self,
        token: Token,
        organisation: crate::types::role::Organisation,
        permissions: crate::types::role::Permissions,
    ) -> Result<()>;
}

pub trait List<F, O, V> {
    fn list(&mut self, filter: F, offset: Option<O>, limit: u32) -> Result<Paginated<V, O>>;
}

pub trait Search<Q, O, V> {
    fn search(&mut self, query: Q, offset: Option<O>, limit: u32) -> Result<Paginated<V, O>>;
}

pub trait Delete<F, T = (), O = ()> {
    fn delete(&mut self, filter: F) -> Result<O>;
}

pub trait Purge<F, T = (), O = ()> {
    fn purge(&mut self, filter: F) -> Result<O>;
}

pub trait Database {
    type Conn;

    fn create<I, O>(&'static self, record: I) -> Result<O>
    where
        Self::Conn: Create<I, O>;

    fn find<K, V>(&'static self, key: K) -> Result<Option<V>>
    where
        Self::Conn: Find<K, V>;

    fn update<K, U, V>(&'static self, key: K, record: U) -> Result<V>
    where
        Self::Conn: Update<K, U, V>;

    fn load<I, O>(&'static self, input: I) -> Result<Vec<O>>
    where
        Self::Conn: Load<I, O>;

    fn authorize(
        &'static self,
        token: Token,
        organisation: crate::types::role::Organisation,
        permissions: crate::types::role::Permissions,
    ) -> Result<()>
    where
        Self::Conn: Authorize;

    fn list<F, O, V>(
        &'static self,
        filter: F,
        offset: Option<O>,
        limit: u32,
    ) -> Result<Paginated<V, O>>
    where
        Self::Conn: List<F, O, V>;

    fn search<Q, O, V>(
        &'static self,
        query: Q,
        offset: Option<O>,
        limit: u32,
    ) -> Result<Paginated<V, O>>
    where
        Self::Conn: Search<Q, O, V>;

    fn delete<F, T, O>(&'static self, filter: F) -> Result<O>
    where
        Self::Conn: Delete<F, T, O>;

    fn purge<F, T, O>(&'static self, filter: F) -> Result<O>
    where
        Self::Conn: Purge<F, T, O>;
}

impl Database for super::Db {
    type Conn = diesel::SqliteConnection;

    fn create<I, O>(&'static self, record: I) -> Result<O>
    where
        Self::Conn: Create<I, O>,
    {
        self.with(|conn| conn.create(record))
    }

    fn find<K, V>(&'static self, key: K) -> Result<Option<V>>
    where
        Self::Conn: Find<K, V>,
    {
        self.with(|conn| conn.find(key))
    }

    fn update<K, U, V>(&'static self, key: K, record: U) -> Result<V>
    where
        Self::Conn: Update<K, U, V>,
    {
        self.with(|conn| conn.update(key, record))
    }

    fn load<I, O>(&'static self, input: I) -> Result<Vec<O>>
    where
        Self::Conn: Load<I, O>,
    {
        self.with(|conn| conn.load(input))
    }

    fn authorize(
        &'static self,
        token: Token,
        organisation: crate::types::role::Organisation,
        permissions: crate::types::role::Permissions,
    ) -> Result<()>
    where
        Self::Conn: Authorize,
    {
        self.with(|conn| conn.authorize(token, organisation, permissions))
    }

    fn list<F, O, V>(
        &'static self,
        filter: F,
        offset: Option<O>,
        limit: u32,
    ) -> Result<Paginated<V, O>>
    where
        Self::Conn: List<F, O, V>,
    {
        self.with(|conn| conn.list(filter, offset, limit))
    }

    fn search<Q, O, V>(
        &'static self,
        query: Q,
        offset: Option<O>,
        limit: u32,
    ) -> Result<Paginated<V, O>>
    where
        Self::Conn: Search<Q, O, V>,
    {
        self.with(|conn| conn.search(query, offset, limit))
    }

    fn delete<F, T, O>(&'static self, filter: F) -> Result<O>
    where
        Self::Conn: Delete<F, T, O>,
    {
        self.with(|conn| conn.delete(filter))
    }

    fn purge<F, T, O>(&'static self, filter: F) -> Result<O>
    where
        Self::Conn: Purge<F, T, O>,
    {
        self.with(|conn| conn.purge(filter))
    }
}
