use crate::types::id::Id;
use crate::types::member::Roles;
use diesel::sql_types::{BigInt, SmallInt, Text};
use diesel::{Queryable, QueryableByName};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Queryable, QueryableByName)]
pub struct Membership {
    #[diesel(sql_type = Text)]
    pub id: Id,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = SmallInt)]
    pub roles: Roles,
    #[diesel(sql_type = BigInt)]
    pub created: i64,
}

impl From<Membership> for crate::proto::types::member::Membership {
    fn from(membership: Membership) -> Self {
        let roles: Vec<i32> = membership.roles.into();
        crate::proto::types::member::Membership {
            id: membership.id.into(),
            name: membership.name,
            roles,
            logo: None,
            created: membership.created,
        }
    }
}
