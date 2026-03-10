#![allow(dead_code)]

use crate::db::schema::{roles, scopes};
use crate::types::id::Id;
use crate::types::paginated::Offset;
use crate::types::role::Permissions;
use diesel::{AsChangeset, Insertable, Queryable, QueryableByName, Selectable};

#[derive(Debug, Clone, PartialEq, Eq, Queryable, QueryableByName, Selectable)]
#[diesel(table_name = roles)]
pub struct Role {
    pub id: Id,
    pub school: Option<Id>,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Permissions,
    pub created: i64,
    pub updated: i64,
}

impl Role {
    pub fn new(
        school: Option<Id>,
        name: String,
        description: Option<String>,
        permissions: Permissions,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: Id::default(),
            school,
            name,
            description,
            permissions,
            created: now,
            updated: now,
        }
    }

    pub fn reference(&self) -> Reference<'_> {
        Reference {
            id: &self.id,
            school: &self.school,
            name: &self.name,
            description: self.description.as_deref(),
            permissions: &self.permissions,
            created: self.created,
            updated: self.updated,
        }
    }

    pub fn system(name: String, description: Option<String>) -> Self {
        Self::new(None, name, description, Permissions::system())
    }
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = roles)]
pub struct Reference<'a> {
    pub id: &'a Id,
    pub school: &'a Option<Id>,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub permissions: &'a Permissions,
    pub created: i64,
    pub updated: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = scopes)]
pub struct Assigner {
    pub school: Option<Id>,
    pub user: Id,
    pub role: Id,
    pub created: i64,
}

impl Assigner {
    pub fn new(school: Option<Id>, user: Id, role: Id) -> Self {
        Self {
            school,
            user,
            role,
            created: chrono::Utc::now().timestamp(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Queryable, QueryableByName)]
pub struct Assignment {
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub school: Option<Id>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub user: Id,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub role: Id,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub created: i64,
}

#[derive(Debug, Clone, Default, AsChangeset)]
#[diesel(table_name = roles)]
pub struct Update {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub permissions: Option<Permissions>,
    pub updated: Option<i64>,
}

impl Offset<i64> for Role {
    fn offset(&self) -> i64 {
        self.created
    }
}
