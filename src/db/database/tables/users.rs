use crate::db::database::traits::{Create, Find, Update};
use crate::db::schema::users;
use crate::types::{
    error::Result,
    id::Id,
    phone::Phone,
    user::{self, User},
};
use diesel::SqliteConnection as Conn;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl};

impl Create<User> for Conn {
    fn create(&mut self, record: User) -> Result<User> {
        let user = diesel::insert_into(users::table)
            .values(record)
            .returning(users::all_columns)
            .get_result(self)?;
        Ok(user)
    }
}

impl Find<Phone, User> for Conn {
    fn find(&mut self, key: Phone) -> Result<Option<User>> {
        let user = users::table
            .filter(users::phone.eq(key))
            .first(self)
            .optional()?;
        Ok(user)
    }
}

impl Find<Id, User> for Conn {
    fn find(&mut self, id: Id) -> Result<Option<User>> {
        let user = users::table.find(id).first(self).optional()?;
        Ok(user)
    }
}

impl Update<Id, user::Update, User> for Conn {
    fn update(&mut self, id: Id, record: user::Update) -> Result<User> {
        let user = diesel::update(users::table.find(id))
            .set(record)
            .returning(users::all_columns)
            .get_result(self)?;
        Ok(user)
    }
}
