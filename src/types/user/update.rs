use super::{Level, Status};
use crate::db::schema::users;
use crate::types::phone::Phone;
use diesel::AsChangeset;

#[derive(Clone, PartialEq, Eq, Hash, AsChangeset, Default)]
#[diesel(table_name = users)]
pub struct Update {
    pub phone: Option<Phone>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub level: Option<Level>,
    pub status: Option<Status>,
    pub created: Option<i64>,
    pub updated: Option<i64>,
}
