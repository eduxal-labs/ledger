use super::{Level, Status};
use crate::config::storage::sign::profile;
use crate::db::schema::users;
use crate::types::error::Result;
use crate::types::id::Id;
use crate::types::phone::Phone;
use diesel::{Insertable, Queryable, QueryableByName, Selectable};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Selectable, QueryableByName, Queryable, Insertable)]
#[diesel(table_name = users)]
pub struct User {
    pub id: Id,
    pub phone: Phone,
    pub email: Option<String>,
    pub name: String,
    pub level: Level,
    pub status: Status,
    pub created: i64,
    pub updated: i64,
}

impl User {
    pub fn invite_super(phone: &str, name: &str, email: Option<&str>) -> Result<Self> {
        let phone = phone.parse()?;
        let name = String::from(name);
        let email = email.map(String::from);
        let user = User {
            id: Id::default(),
            phone,
            email,
            name,
            level: Level::Super,
            status: Status::Invited,
            created: chrono::Utc::now().timestamp(),
            updated: chrono::Utc::now().timestamp(),
        };
        Ok(user)
    }

    pub fn new(phone: Phone, name: String) -> Self {
        let id = Id::default();
        let email = None;
        let level = Level::Normal;
        let status = Status::Active;
        let created = chrono::Utc::now().timestamp();
        let updated = chrono::Utc::now().timestamp();
        User {
            id,
            phone,
            email,
            name,
            level,
            status,
            created,
            updated,
        }
    }
}

impl From<User> for crate::proto::types::user::User {
    fn from(user: User) -> Self {
        let id = user.id.into();
        let name = user.name;
        let email = user.email;
        let phone = user.phone.into();
        let level = user.level.into();
        let status = user.status.into();
        let created = user.created;
        let updated = user.updated;
        let profile = profile(&user.id, None, false);

        crate::proto::types::user::User {
            id,
            name,
            email,
            phone,
            level,
            status,
            created,
            updated,
            profile,
        }
    }
}
