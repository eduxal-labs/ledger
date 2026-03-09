use super::{Action, Actions, Resource};
use crate::types::error::Error;
use diesel::deserialize::FromSqlRow;
use diesel::expression::AsExpression;
use diesel::sql_types::Binary;
use std::ops::{Add, AddAssign, Index, IndexMut, Sub, SubAssign};

type ProtoPermission = crate::proto::types::role::Permission;
type ProtoResource = crate::proto::types::role::Resource;
type ProtoAction = crate::proto::types::role::Action;

#[derive(AsExpression, FromSqlRow)]
#[diesel(sql_type = Binary)]
#[derive(Debug, Clone, Copy, Eq)]
pub struct Permissions([Actions; Resource::COUNT]);

impl Permissions {
    pub fn new() -> Self {
        Self([Actions::default(); Resource::COUNT])
    }

    pub fn system() -> Self {
        let mut permissions = Self::new();
        let read = Actions::from(Action::Read);
        permissions[Resource::Users] = read;
        permissions[Resource::Schools] = read;
        permissions[Resource::Owners] = read;
        permissions[Resource::Teachers] = read;
        permissions[Resource::Staff] = read;
        permissions[Resource::Students] = read;
        permissions[Resource::Departments] = read;
        permissions[Resource::Classes] = read;
        permissions[Resource::Roles] = read;
        permissions[Resource::Plans] = read;
        permissions
    }

    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|a| a.is_empty())
    }
}

impl Default for Permissions {
    fn default() -> Self {
        Self::new()
    }
}

impl TryFrom<&[u8]> for Permissions {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() % 3 != 0 {
            return Err(Error::InvalidPermissions);
        }

        let mut permissions = Self::new();

        for chunk in bytes.chunks_exact(3) {
            let resource = Resource::try_from(chunk[0]).map_err(|_| Error::InvalidPermissions)?;
            let actions = Actions::new(u16::from_le_bytes([chunk[1], chunk[2]]));
            permissions[resource] = actions;
        }

        Ok(permissions)
    }
}

impl From<&Permissions> for Vec<u8> {
    fn from(permissions: &Permissions) -> Self {
        let mut bytes = Vec::new();

        for resource in Resource::VARIANTS {
            let actions = permissions[resource];
            if !actions.is_empty() {
                let bits = actions.bits().to_le_bytes();
                bytes.push(u8::from(resource));
                bytes.push(bits[0]);
                bytes.push(bits[1]);
            }
        }

        bytes
    }
}

impl Index<Resource> for Permissions {
    type Output = Actions;

    fn index(&self, resource: Resource) -> &Self::Output {
        &self.0[usize::from(resource)]
    }
}

impl IndexMut<Resource> for Permissions {
    fn index_mut(&mut self, resource: Resource) -> &mut Self::Output {
        &mut self.0[usize::from(resource)]
    }
}

impl Add for Permissions {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut result = self;
        for resource in Resource::VARIANTS {
            result[resource] = result[resource] + rhs[resource];
        }
        result
    }
}

impl AddAssign for Permissions {
    fn add_assign(&mut self, rhs: Self) {
        for resource in Resource::VARIANTS {
            self[resource] += rhs[resource];
        }
    }
}

impl Sub for Permissions {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut result = self;
        for resource in Resource::VARIANTS {
            result[resource] = result[resource] - rhs[resource];
        }
        result
    }
}

impl SubAssign for Permissions {
    fn sub_assign(&mut self, rhs: Self) {
        for resource in Resource::VARIANTS {
            self[resource] -= rhs[resource];
        }
    }
}

impl PartialEq for Permissions {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialEq<Actions> for Permissions {
    fn eq(&self, other: &Actions) -> bool {
        self.0.iter().all(|a| *a == *other)
    }
}

impl diesel::serialize::ToSql<Binary, diesel::sqlite::Sqlite> for Permissions {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        let bytes: Vec<u8> = self.into();
        out.set_value(bytes);
        Ok(diesel::serialize::IsNull::No)
    }
}

impl diesel::deserialize::FromSql<Binary, diesel::sqlite::Sqlite> for Permissions {
    fn from_sql(
        bytes: <diesel::sqlite::Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        let blob = <*const [u8] as diesel::deserialize::FromSql<Binary, diesel::sqlite::Sqlite>>::from_sql(bytes)?;
        let slice = unsafe { &*blob };
        Permissions::try_from(slice).map_err(|e| e.to_string().into())
    }
}

impl From<&Permissions> for Vec<ProtoPermission> {
    fn from(permissions: &Permissions) -> Self {
        Resource::VARIANTS
            .into_iter()
            .filter(|r| !permissions[*r].is_empty())
            .map(|resource| {
                let actions: Vec<i32> = permissions[resource].into();
                ProtoPermission {
                    resource: ProtoResource::from(resource) as i32,
                    actions,
                }
            })
            .collect()
    }
}

impl From<Permissions> for Vec<ProtoPermission> {
    fn from(permissions: Permissions) -> Self {
        (&permissions).into()
    }
}

impl TryFrom<&[ProtoPermission]> for Permissions {
    type Error = Error;

    fn try_from(proto: &[ProtoPermission]) -> Result<Self, Self::Error> {
        let mut permissions = Self::new();

        for permission in proto {
            let proto_resource =
                ProtoResource::try_from(permission.resource).map_err(|_| Error::InvalidResource)?;
            let resource = Resource::from(proto_resource);

            let mut actions = Actions::default();
            for &action_i32 in &permission.actions {
                let proto_action =
                    ProtoAction::try_from(action_i32).map_err(|_| Error::InvalidAction)?;
                actions += Action::from(proto_action);
            }

            permissions[resource] = actions;
        }

        Ok(permissions)
    }
}

impl From<crate::types::role::Role> for crate::proto::types::role::Role {
    fn from(role: crate::types::role::Role) -> Self {
        let permissions: Vec<ProtoPermission> = (&role.permissions).into();
        crate::proto::types::role::Role {
            id: role.id.into(),
            name: role.name,
            permissions,
            created: role.created,
        }
    }
}

impl From<crate::types::role::Assignment> for crate::proto::types::role::Assignment {
    fn from(assignment: crate::types::role::Assignment) -> Self {
        crate::proto::types::role::Assignment {
            id: assignment.role.into(),
            name: String::new(),
            assigned: assignment.created,
            profile: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        let permissions = Permissions::new();
        assert!(permissions.is_empty());
        assert_eq!(permissions, Actions::default());
    }

    #[test]
    fn default_is_empty() {
        let permissions = Permissions::default();
        assert!(permissions.is_empty());
    }

    #[test]
    fn index_and_index_mut() {
        let mut permissions = Permissions::new();
        assert!(permissions[Resource::Users].is_empty());

        permissions[Resource::Users] = Actions::from(Action::Read);
        assert!(permissions[Resource::Users].contains(Action::Read));
        assert!(!permissions[Resource::Users].contains(Action::Create));
    }

    #[test]
    fn not_empty_after_set() {
        let mut permissions = Permissions::new();
        permissions[Resource::Schools] = Actions::from(Action::Create);
        assert!(!permissions.is_empty());
    }

    #[test]
    fn system_constructor() {
        let system = Permissions::system();
        assert!(!system.is_empty());

        let read_resources = [
            Resource::Users,
            Resource::Schools,
            Resource::Owners,
            Resource::Teachers,
            Resource::Staff,
            Resource::Students,
            Resource::Departments,
            Resource::Classes,
            Resource::Roles,
            Resource::Plans,
        ];

        for resource in read_resources {
            assert!(
                system[resource].contains(Action::Read),
                "system() should grant Read on {:?}",
                resource
            );
            assert_eq!(
                system[resource].len(),
                1,
                "system() should only grant Read on {:?}",
                resource
            );
        }

        let no_access_resources = [
            Resource::Attendance,
            Resource::Lessons,
            Resource::Exams,
            Resource::Grades,
            Resource::Fees,
            Resource::Payments,
            Resource::Announcements,
            Resource::AI,
        ];

        for resource in no_access_resources {
            assert!(
                system[resource].is_empty(),
                "system() should not grant access on {:?}",
                resource
            );
        }
    }

    #[test]
    fn binary_roundtrip_empty() {
        let permissions = Permissions::new();
        let bytes: Vec<u8> = (&permissions).into();
        assert!(bytes.is_empty());
        let decoded = Permissions::try_from(bytes.as_slice()).unwrap();
        assert_eq!(decoded, permissions);
    }

    #[test]
    fn binary_roundtrip_single() {
        let mut permissions = Permissions::new();
        permissions[Resource::Users] = Actions::from(Action::Read) + Action::Update;

        let bytes: Vec<u8> = (&permissions).into();
        assert_eq!(bytes.len(), 3);

        assert_eq!(bytes[0], u8::from(Resource::Users));
        let actions_bits = u16::from_le_bytes([bytes[1], bytes[2]]);
        assert_eq!(actions_bits, Action::Read as u16 | Action::Update as u16);

        let decoded = Permissions::try_from(bytes.as_slice()).unwrap();
        assert_eq!(decoded, permissions);
    }

    #[test]
    fn binary_roundtrip_multiple() {
        let mut permissions = Permissions::new();
        permissions[Resource::Users] = Actions::from(Action::Read);
        permissions[Resource::Schools] =
            Actions::from(Action::Create) + Action::Read + Action::Update;
        permissions[Resource::Roles] = Actions::from(Action::Assign) + Action::Unassign;

        let bytes: Vec<u8> = (&permissions).into();
        assert_eq!(bytes.len(), 9); // 3 resources × 3 bytes

        let decoded = Permissions::try_from(bytes.as_slice()).unwrap();
        assert_eq!(decoded, permissions);
    }

    #[test]
    fn binary_roundtrip_all_resources() {
        let mut permissions = Permissions::new();
        for resource in Resource::VARIANTS {
            permissions[resource] = Actions::from(Action::Read);
        }

        let bytes: Vec<u8> = (&permissions).into();
        assert_eq!(bytes.len(), 18 * 3); // 18 resources × 3 bytes

        let decoded = Permissions::try_from(bytes.as_slice()).unwrap();
        assert_eq!(decoded, permissions);
    }

    #[test]
    fn try_from_invalid_length() {
        let bytes = vec![1u8, 2]; // 2 bytes, not a multiple of 3
        assert!(Permissions::try_from(bytes.as_slice()).is_err());

        let bytes = vec![1u8, 2, 0, 3]; // 4 bytes, not a multiple of 3
        assert!(Permissions::try_from(bytes.as_slice()).is_err());
    }

    #[test]
    fn try_from_invalid_resource() {
        let bytes = vec![0u8, 2, 0]; // resource 0 doesn't exist (starts at 1)
        assert!(Permissions::try_from(bytes.as_slice()).is_err());

        let bytes = vec![19u8, 2, 0]; // resource 19 doesn't exist
        assert!(Permissions::try_from(bytes.as_slice()).is_err());
    }

    #[test]
    fn add_permissions() {
        let mut a = Permissions::new();
        a[Resource::Users] = Actions::from(Action::Read);
        a[Resource::Schools] = Actions::from(Action::Create);

        let mut b = Permissions::new();
        b[Resource::Users] = Actions::from(Action::Update);
        b[Resource::Roles] = Actions::from(Action::Assign);

        let combined = a + b;
        assert!(combined[Resource::Users].contains(Action::Read));
        assert!(combined[Resource::Users].contains(Action::Update));
        assert!(combined[Resource::Schools].contains(Action::Create));
        assert!(combined[Resource::Roles].contains(Action::Assign));
    }

    #[test]
    fn add_assign_permissions() {
        let mut a = Permissions::new();
        a[Resource::Users] = Actions::from(Action::Read);

        let mut b = Permissions::new();
        b[Resource::Users] = Actions::from(Action::Update);
        b[Resource::Schools] = Actions::from(Action::Create);

        a += b;
        assert!(a[Resource::Users].contains(Action::Read));
        assert!(a[Resource::Users].contains(Action::Update));
        assert!(a[Resource::Schools].contains(Action::Create));
    }

    #[test]
    fn sub_permissions() {
        let mut a = Permissions::new();
        a[Resource::Users] = Actions::from(Action::Read) + Action::Update + Action::Delete;
        a[Resource::Schools] = Actions::from(Action::Create);

        let mut b = Permissions::new();
        b[Resource::Users] = Actions::from(Action::Update) + Action::Delete;

        let result = a - b;
        assert!(result[Resource::Users].contains(Action::Read));
        assert!(!result[Resource::Users].contains(Action::Update));
        assert!(!result[Resource::Users].contains(Action::Delete));
        assert!(result[Resource::Schools].contains(Action::Create));
    }

    #[test]
    fn sub_assign_permissions() {
        let mut a = Permissions::new();
        a[Resource::Users] = Actions::from(Action::Read) + Action::Update;

        let mut b = Permissions::new();
        b[Resource::Users] = Actions::from(Action::Read);

        a -= b;
        assert!(!a[Resource::Users].contains(Action::Read));
        assert!(a[Resource::Users].contains(Action::Update));
    }

    #[test]
    fn partial_eq_actions() {
        let permissions = Permissions::new();
        assert_eq!(permissions, Actions::default());

        let mut permissions = Permissions::new();
        permissions[Resource::Users] = Actions::from(Action::Read);
        assert_ne!(permissions, Actions::default());
    }

    #[test]
    fn sub_yields_empty_for_authorization() {
        // Simulates authorization check: required - granted = remaining
        // If remaining is empty, the user has all required permissions
        let mut required = Permissions::new();
        required[Resource::Users] = Actions::from(Action::Read);
        required[Resource::Schools] = Actions::from(Action::Create) + Action::Read;

        let mut granted = Permissions::new();
        granted[Resource::Users] = Actions::from(Action::Read) + Action::Update;
        granted[Resource::Schools] = Actions::from(Action::Create) + Action::Read + Action::Delete;

        let remaining = required - granted;
        assert!(remaining.is_empty());
    }

    #[test]
    fn sub_yields_nonempty_for_insufficient() {
        let mut required = Permissions::new();
        required[Resource::Users] = Actions::from(Action::Read) + Action::Update;

        let mut granted = Permissions::new();
        granted[Resource::Users] = Actions::from(Action::Read);

        let remaining = required - granted;
        assert!(!remaining.is_empty());
        assert!(remaining[Resource::Users].contains(Action::Update));
        assert!(!remaining[Resource::Users].contains(Action::Read));
    }

    #[test]
    fn system_binary_roundtrip() {
        let system = Permissions::system();
        let bytes: Vec<u8> = (&system).into();
        let decoded = Permissions::try_from(bytes.as_slice()).unwrap();
        assert_eq!(decoded, system);
    }

    #[test]
    fn binary_preserves_resource_order() {
        let mut permissions = Permissions::new();
        permissions[Resource::AI] = Actions::from(Action::Read);
        permissions[Resource::Users] = Actions::from(Action::Create);

        let bytes: Vec<u8> = (&permissions).into();
        assert_eq!(bytes.len(), 6);

        // Users (1) should come before AI (18) in serialized form
        assert_eq!(bytes[0], u8::from(Resource::Users));
        assert_eq!(bytes[3], u8::from(Resource::AI));
    }

    #[test]
    fn empty_bytes_roundtrip() {
        let bytes: Vec<u8> = vec![];
        let permissions = Permissions::try_from(bytes.as_slice()).unwrap();
        assert!(permissions.is_empty());
    }

    #[test]
    fn actions_bits_preserved_in_binary() {
        let mut permissions = Permissions::new();
        // Use all 9 actions on one resource
        let mut all_actions = Actions::default();
        for action in Action::VARIANTS {
            all_actions += action;
        }
        permissions[Resource::Users] = all_actions;

        let bytes: Vec<u8> = (&permissions).into();
        assert_eq!(bytes.len(), 3);

        let decoded = Permissions::try_from(bytes.as_slice()).unwrap();
        assert_eq!(decoded[Resource::Users], all_actions);
        for action in Action::VARIANTS {
            assert!(decoded[Resource::Users].contains(action));
        }
    }

    #[test]
    fn proto_roundtrip_empty() {
        let permissions = Permissions::new();
        let proto: Vec<ProtoPermission> = (&permissions).into();
        assert!(proto.is_empty());
        let decoded = Permissions::try_from(proto.as_slice()).unwrap();
        assert_eq!(decoded, permissions);
    }

    #[test]
    fn proto_roundtrip_single() {
        let mut permissions = Permissions::new();
        permissions[Resource::Users] = Actions::from(Action::Read) + Action::Update;

        let proto: Vec<ProtoPermission> = (&permissions).into();
        assert_eq!(proto.len(), 1);
        assert_eq!(proto[0].resource, ProtoResource::Users as i32);
        assert_eq!(proto[0].actions.len(), 2);

        let decoded = Permissions::try_from(proto.as_slice()).unwrap();
        assert_eq!(decoded, permissions);
    }

    #[test]
    fn proto_roundtrip_multiple() {
        let mut permissions = Permissions::new();
        permissions[Resource::Users] = Actions::from(Action::Read);
        permissions[Resource::Schools] =
            Actions::from(Action::Create) + Action::Read + Action::Update;
        permissions[Resource::Roles] = Actions::from(Action::Assign) + Action::Unassign;

        let proto: Vec<ProtoPermission> = (&permissions).into();
        assert_eq!(proto.len(), 3);

        let decoded = Permissions::try_from(proto.as_slice()).unwrap();
        assert_eq!(decoded, permissions);
    }

    #[test]
    fn proto_roundtrip_all_resources() {
        let mut permissions = Permissions::new();
        for resource in Resource::VARIANTS {
            permissions[resource] = Actions::from(Action::Read);
        }

        let proto: Vec<ProtoPermission> = (&permissions).into();
        assert_eq!(proto.len(), 18);

        let decoded = Permissions::try_from(proto.as_slice()).unwrap();
        assert_eq!(decoded, permissions);
    }

    #[test]
    fn proto_invalid_resource() {
        let proto = vec![ProtoPermission {
            resource: 99,
            actions: vec![ProtoAction::Read as i32],
        }];
        assert!(Permissions::try_from(proto.as_slice()).is_err());
    }

    #[test]
    fn proto_invalid_action() {
        let proto = vec![ProtoPermission {
            resource: ProtoResource::Users as i32,
            actions: vec![99],
        }];
        assert!(Permissions::try_from(proto.as_slice()).is_err());
    }

    #[test]
    fn proto_system_roundtrip() {
        let system = Permissions::system();
        let proto: Vec<ProtoPermission> = (&system).into();
        let decoded = Permissions::try_from(proto.as_slice()).unwrap();
        assert_eq!(decoded, system);
    }

    #[test]
    fn proto_from_owned() {
        let mut permissions = Permissions::new();
        permissions[Resource::AI] = Actions::from(Action::Read);
        let proto: Vec<ProtoPermission> = permissions.into();
        assert_eq!(proto.len(), 1);
    }
}
