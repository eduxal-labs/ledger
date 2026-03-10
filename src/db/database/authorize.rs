#![allow(dead_code)]

use crate::db::database::traits::{Authorize, Load};
use crate::db::schema::{owners, roles, schools, scopes, users};
use crate::types::error::{Error, Result};
use crate::types::id::Id;
use crate::types::role::{Organisation, Permissions, Role};
use crate::types::token::Token;
use crate::types::user::{Level, Status, User};
use diesel::{
    ExpressionMethods, JoinOnDsl, OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection,
};
use tracing::{debug, warn};

type Conn = SqliteConnection;

// Load school-scoped roles for a user: (school_id, &User) -> Vec<Role>
impl Load<(Id, &User), Role> for Conn {
    fn load(&mut self, (school, user): (Id, &User)) -> Result<Vec<Role>> {
        let results = roles::table
            .inner_join(scopes::table.on(scopes::role.eq(roles::id)))
            .filter(scopes::user.eq(user.id))
            .filter(scopes::school.eq(school))
            .select(roles::all_columns)
            .load::<Role>(self)?;
        Ok(results)
    }
}

// Load system-scoped roles for a user: &User -> Vec<Role>
impl Load<&User, Role> for Conn {
    fn load(&mut self, user: &User) -> Result<Vec<Role>> {
        let results = roles::table
            .inner_join(scopes::table.on(scopes::role.eq(roles::id)))
            .filter(scopes::user.eq(user.id))
            .filter(scopes::school.is_null())
            .select(roles::all_columns)
            .load::<Role>(self)?;
        Ok(results)
    }
}

// Combined loader: (Option<Id>, &User) -> Vec<Role>
// None = system-scoped, Some(id) = school-scoped
impl Load<(Option<Id>, &User), Role> for Conn {
    fn load(&mut self, (school, user): (Option<Id>, &User)) -> Result<Vec<Role>> {
        match school {
            Some(id) => self.load((id, user)),
            None => self.load(user),
        }
    }
}

impl Authorize for Conn {
    fn authorize(
        &mut self,
        token: Token,
        organisation: Organisation,
        permissions: Permissions,
    ) -> Result<()> {
        // 1. Load the user
        let user: User = users::table
            .find(token.user)
            .first(self)
            .optional()?
            .ok_or(Error::UserNotFound)?;

        // 2. Check user is active
        if user.status != Status::Active {
            return Err(Error::Forbidden);
        }

        // 3. Super users bypass all checks
        if user.level == Level::Super {
            debug!(user_id = %user.id, "authorize: super user bypass");
            return Ok(());
        }

        match organisation {
            Organisation::System => {
                // System context: only system-level users can operate here
                if user.level != Level::System {
                    return Err(Error::Forbidden);
                }

                let roles: Vec<Role> = self.load(&user)?;
                let granted = aggregate_permissions(&roles);
                check_permissions(permissions, granted)
            }

            Organisation::Account => {
                // Account context: user can only act on their own account
                // No role-based checks — the service layer validates ownership
                Ok(())
            }

            Organisation::School(school_id) => {
                // Verify the school exists and is active
                let school_status: i16 = schools::table
                    .find(school_id)
                    .select(schools::status)
                    .first(self)
                    .optional()?
                    .ok_or(Error::SchoolNotFound)?;

                // School status: Active = 1
                if school_status != 1 {
                    return Err(Error::Forbidden);
                }

                // Check if user is an owner — owners bypass permission checks
                // within their school
                let is_owner: bool = owners::table
                    .filter(owners::school.eq(school_id))
                    .filter(owners::user.eq(user.id))
                    .first::<(Id, Id, i64)>(self)
                    .optional()?
                    .is_some();

                if is_owner {
                    debug!(user_id = %user.id, school_id = %school_id, "authorize: owner bypass");
                    return Ok(());
                }

                // Load school-scoped roles
                let mut roles: Vec<Role> = self.load((school_id, &user))?;

                // System users also get their system-scoped roles merged in
                if user.level == Level::System {
                    let system_roles: Vec<Role> = self.load(&user)?;
                    roles.extend(system_roles);
                }

                let granted = aggregate_permissions(&roles);
                check_permissions(permissions, granted)
            }
        }
    }
}

fn aggregate_permissions(roles: &[Role]) -> Permissions {
    let mut granted = Permissions::new();
    for role in roles {
        granted += role.permissions;
    }
    granted
}

fn check_permissions(required: Permissions, granted: Permissions) -> Result<()> {
    let remaining = required - granted;
    if remaining.is_empty() {
        Ok(())
    } else {
        warn!("authorize: forbidden — required permissions not satisfied");
        Err(Error::Forbidden)
    }
}
