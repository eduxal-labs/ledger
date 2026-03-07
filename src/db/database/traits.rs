use crate::types::error::Result;
use std::cell::RefCell;
use std::thread::LocalKey;

pub trait Create<I, O = I> {
    fn create(&mut self, record: I) -> Result<O>;
}

pub trait Find<K, V> {
    fn find(&mut self, key: K) -> Result<Option<V>>;
}

pub trait Update<K, U, V = ()> {
    fn update(&mut self, key: K, record: U) -> Result<V>;
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
}

impl<T> Database for LocalKey<RefCell<T>> {
    type Conn = T;

    fn create<I, O>(&'static self, record: I) -> Result<O>
    where
        Self::Conn: Create<I, O>,
    {
        self.with(|cell| cell.borrow_mut().create(record))
    }

    fn find<K, V>(&'static self, key: K) -> Result<Option<V>>
    where
        Self::Conn: Find<K, V>,
    {
        self.with(|cell| cell.borrow_mut().find(key))
    }

    fn update<K, U, V>(&'static self, key: K, record: U) -> Result<V>
    where
        Self::Conn: Update<K, U, V>,
    {
        self.with(|cell| cell.borrow_mut().update(key, record))
    }
}
