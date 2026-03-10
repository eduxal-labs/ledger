#![allow(dead_code)]

pub struct Paginated<T, O> {
    pub items: Vec<T>,
    pub next: Option<O>,
}

pub trait Offset<O: Copy> {
    fn offset(&self) -> O;
}
