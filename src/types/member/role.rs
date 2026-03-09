use crate::types::error::Error;
use diesel::sql_types::SmallInt;
use diesel::{AsExpression, FromSqlRow};
use std::ops::{Add, AddAssign, Sub, SubAssign};

type ProtoRole = crate::proto::types::member::Role;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Role {
    Owner = 1,
    Guardian = 2,
    Student = 4,
    Teacher = 8,
    Staff = 16,
}

impl Role {
    pub const VARIANTS: [Self; 5] = [
        Self::Owner,
        Self::Guardian,
        Self::Student,
        Self::Teacher,
        Self::Staff,
    ];
}

impl TryFrom<u8> for Role {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Role::Owner),
            2 => Ok(Role::Guardian),
            4 => Ok(Role::Student),
            8 => Ok(Role::Teacher),
            16 => Ok(Role::Staff),
            _ => Err(Error::InvalidRole),
        }
    }
}

impl TryFrom<i32> for Role {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value < 0 || value > u8::MAX as i32 {
            return Err(Error::InvalidRole);
        }
        (value as u8).try_into()
    }
}

impl From<Role> for u8 {
    fn from(role: Role) -> Self {
        role as u8
    }
}

impl From<Role> for i32 {
    fn from(role: Role) -> Self {
        role as u8 as i32
    }
}

impl From<Role> for ProtoRole {
    fn from(role: Role) -> Self {
        match role {
            Role::Owner => ProtoRole::Owner,
            Role::Guardian => ProtoRole::Guardian,
            Role::Student => ProtoRole::Student,
            Role::Teacher => ProtoRole::Teacher,
            Role::Staff => ProtoRole::Staff,
        }
    }
}

impl From<ProtoRole> for Role {
    fn from(role: ProtoRole) -> Self {
        match role {
            ProtoRole::Owner => Role::Owner,
            ProtoRole::Guardian => Role::Guardian,
            ProtoRole::Student => Role::Student,
            ProtoRole::Teacher => Role::Teacher,
            ProtoRole::Staff => Role::Staff,
        }
    }
}

impl From<Roles> for Vec<i32> {
    fn from(roles: Roles) -> Self {
        roles.iter().map(|r| ProtoRole::from(r) as i32).collect()
    }
}

#[derive(AsExpression, FromSqlRow)]
#[diesel(sql_type = SmallInt)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Roles(u8);

const ALL_BITS: u8 = Role::Owner as u8
    | Role::Guardian as u8
    | Role::Student as u8
    | Role::Teacher as u8
    | Role::Staff as u8;

const ALL_ROLES: [Role; 5] = [
    Role::Owner,
    Role::Guardian,
    Role::Student,
    Role::Teacher,
    Role::Staff,
];

impl Roles {
    pub fn new(bits: u8) -> Self {
        Self(bits & ALL_BITS)
    }

    pub fn bits(self) -> u8 {
        self.0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    pub fn contains(self, role: Role) -> bool {
        self.0 & role as u8 != 0
    }

    pub fn iter(self) -> impl Iterator<Item = Role> {
        let bits = self.0;
        ALL_ROLES.into_iter().filter(move |r| bits & *r as u8 != 0)
    }
}

impl From<Role> for Roles {
    fn from(role: Role) -> Self {
        Self(role as u8)
    }
}

impl TryFrom<i32> for Roles {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value < 0 || value > u8::MAX as i32 {
            return Err(Error::InvalidRole);
        }
        let bits = value as u8;
        if bits & !ALL_BITS != 0 {
            return Err(Error::InvalidRole);
        }
        Ok(Self(bits))
    }
}

impl From<Roles> for i32 {
    fn from(roles: Roles) -> Self {
        roles.0 as i32
    }
}

impl Add for Roles {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl Add<Role> for Roles {
    type Output = Self;

    fn add(self, rhs: Role) -> Self::Output {
        Self(self.0 | rhs as u8)
    }
}

impl AddAssign for Roles {
    fn add_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl AddAssign<Role> for Roles {
    fn add_assign(&mut self, rhs: Role) {
        self.0 |= rhs as u8;
    }
}

impl Sub for Roles {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 & !rhs.0)
    }
}

impl Sub<Role> for Roles {
    type Output = Self;

    fn sub(self, rhs: Role) -> Self::Output {
        Self(self.0 & !(rhs as u8))
    }
}

impl SubAssign for Roles {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 &= !rhs.0;
    }
}

impl SubAssign<Role> for Roles {
    fn sub_assign(&mut self, rhs: Role) {
        self.0 &= !(rhs as u8);
    }
}

impl diesel::serialize::ToSql<SmallInt, diesel::sqlite::Sqlite> for Roles {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        out.set_value(self.0 as i32);
        Ok(diesel::serialize::IsNull::No)
    }
}

impl diesel::deserialize::FromSql<SmallInt, diesel::sqlite::Sqlite> for Roles {
    fn from_sql(
        bytes: <diesel::sqlite::Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        let value =
            <i16 as diesel::deserialize::FromSql<SmallInt, diesel::sqlite::Sqlite>>::from_sql(
                bytes,
            )?;
        if value < 0 || value > u8::MAX as i16 {
            return Err("invalid roles bitmask".into());
        }
        let bits = value as u8;
        if bits & !ALL_BITS != 0 {
            return Err("invalid roles bitmask".into());
        }
        Ok(Self(bits))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmask_values() {
        assert_eq!(Role::Owner as u8, 1);
        assert_eq!(Role::Guardian as u8, 2);
        assert_eq!(Role::Student as u8, 4);
        assert_eq!(Role::Teacher as u8, 8);
        assert_eq!(Role::Staff as u8, 16);
    }

    #[test]
    fn all_powers_of_two() {
        for role in Role::VARIANTS {
            let v = role as u8;
            assert!(
                v.is_power_of_two(),
                "{:?} = {} is not a power of two",
                role,
                v
            );
        }
    }

    #[test]
    fn variants_count() {
        assert_eq!(Role::VARIANTS.len(), 5);
    }

    #[test]
    fn try_from_u8_valid() {
        for role in Role::VARIANTS {
            let value = role as u8;
            assert_eq!(Role::try_from(value).unwrap(), role);
        }
    }

    #[test]
    fn try_from_u8_invalid() {
        assert!(Role::try_from(0u8).is_err());
        assert!(Role::try_from(3u8).is_err());
        assert!(Role::try_from(32u8).is_err());
    }

    #[test]
    fn try_from_i32_valid() {
        for role in Role::VARIANTS {
            let value = role as u8 as i32;
            assert_eq!(Role::try_from(value).unwrap(), role);
        }
    }

    #[test]
    fn try_from_i32_invalid() {
        assert!(Role::try_from(-1i32).is_err());
        assert!(Role::try_from(0i32).is_err());
        assert!(Role::try_from(3i32).is_err());
        assert!(Role::try_from(300i32).is_err());
    }

    #[test]
    fn roundtrip_u8() {
        for role in Role::VARIANTS {
            let value: u8 = role.into();
            assert_eq!(Role::try_from(value).unwrap(), role);
        }
    }

    #[test]
    fn roundtrip_i32() {
        for role in Role::VARIANTS {
            let value: i32 = role.into();
            assert_eq!(Role::try_from(value).unwrap(), role);
        }
    }

    #[test]
    fn default_is_empty() {
        let roles = Roles::default();
        assert!(roles.is_empty());
        assert_eq!(roles.len(), 0);
        assert_eq!(roles.bits(), 0);
    }

    #[test]
    fn from_role() {
        let roles = Roles::from(Role::Teacher);
        assert!(roles.contains(Role::Teacher));
        assert!(!roles.contains(Role::Owner));
        assert_eq!(roles.len(), 1);
    }

    #[test]
    fn add_roles() {
        let a = Roles::from(Role::Owner);
        let b = Roles::from(Role::Teacher);
        let combined = a + b;
        assert!(combined.contains(Role::Owner));
        assert!(combined.contains(Role::Teacher));
        assert_eq!(combined.len(), 2);
    }

    #[test]
    fn add_role() {
        let roles = Roles::default() + Role::Owner + Role::Staff;
        assert!(roles.contains(Role::Owner));
        assert!(roles.contains(Role::Staff));
        assert!(!roles.contains(Role::Student));
        assert_eq!(roles.len(), 2);
    }

    #[test]
    fn add_assign_roles() {
        let mut roles = Roles::from(Role::Owner);
        roles += Roles::from(Role::Guardian);
        assert!(roles.contains(Role::Owner));
        assert!(roles.contains(Role::Guardian));
    }

    #[test]
    fn add_assign_role() {
        let mut roles = Roles::default();
        roles += Role::Student;
        roles += Role::Teacher;
        assert!(roles.contains(Role::Student));
        assert!(roles.contains(Role::Teacher));
        assert_eq!(roles.len(), 2);
    }

    #[test]
    fn sub_roles() {
        let all = Roles::from(Role::Owner) + Role::Teacher + Role::Staff;
        let removed = all - Roles::from(Role::Teacher);
        assert!(removed.contains(Role::Owner));
        assert!(!removed.contains(Role::Teacher));
        assert!(removed.contains(Role::Staff));
    }

    #[test]
    fn sub_role() {
        let roles = Roles::from(Role::Owner) + Role::Guardian;
        let result = roles - Role::Owner;
        assert!(!result.contains(Role::Owner));
        assert!(result.contains(Role::Guardian));
    }

    #[test]
    fn sub_assign_roles() {
        let mut roles = Roles::from(Role::Owner) + Role::Teacher + Role::Staff;
        roles -= Roles::from(Role::Teacher);
        assert!(!roles.contains(Role::Teacher));
        assert_eq!(roles.len(), 2);
    }

    #[test]
    fn sub_assign_role() {
        let mut roles = Roles::from(Role::Student) + Role::Guardian;
        roles -= Role::Student;
        assert!(!roles.contains(Role::Student));
        assert!(roles.contains(Role::Guardian));
    }

    #[test]
    fn idempotent_add() {
        let roles = Roles::from(Role::Owner) + Role::Owner;
        assert_eq!(roles.len(), 1);
    }

    #[test]
    fn sub_absent_is_noop() {
        let roles = Roles::from(Role::Teacher);
        let result = roles - Role::Staff;
        assert_eq!(result, roles);
    }

    #[test]
    fn iter_all() {
        let mut roles = Roles::default();
        for role in ALL_ROLES {
            roles += role;
        }
        let collected: Vec<Role> = roles.iter().collect();
        assert_eq!(collected.len(), 5);
        assert_eq!(collected, ALL_ROLES.to_vec());
    }

    #[test]
    fn iter_subset() {
        let roles = Roles::from(Role::Guardian) + Role::Staff;
        let collected: Vec<Role> = roles.iter().collect();
        assert_eq!(collected, vec![Role::Guardian, Role::Staff]);
    }

    #[test]
    fn new_masks_invalid_bits() {
        let roles = Roles::new(0xFF);
        assert_eq!(roles.bits(), ALL_BITS);
        assert_eq!(roles.len(), 5);
    }

    #[test]
    fn contains_all_five_roles() {
        let mut roles = Roles::default();
        roles += Role::Owner;
        roles += Role::Guardian;
        roles += Role::Student;
        roles += Role::Teacher;
        roles += Role::Staff;
        assert_eq!(roles.len(), 5);
        assert!(!roles.is_empty());
        for role in ALL_ROLES {
            assert!(roles.contains(role));
        }
    }

    #[test]
    fn roles_try_from_i32_valid() {
        let roles = Roles::try_from(9i32).unwrap(); // Owner(1) | Teacher(8)
        assert!(roles.contains(Role::Owner));
        assert!(roles.contains(Role::Teacher));
        assert_eq!(roles.len(), 2);
    }

    #[test]
    fn roles_try_from_i32_zero() {
        let roles = Roles::try_from(0i32).unwrap();
        assert!(roles.is_empty());
    }

    #[test]
    fn roles_try_from_i32_invalid() {
        assert!(Roles::try_from(-1i32).is_err());
        assert!(Roles::try_from(32i32).is_err()); // bit 5 is not a valid role
        assert!(Roles::try_from(64i32).is_err());
        assert!(Roles::try_from(256i32).is_err());
    }

    #[test]
    fn roles_to_i32() {
        let roles = Roles::from(Role::Owner) + Role::Staff;
        let value: i32 = roles.into();
        assert_eq!(value, 17); // 1 | 16
    }

    #[test]
    fn roles_i32_roundtrip() {
        let roles = Roles::from(Role::Guardian) + Role::Student + Role::Teacher;
        let value: i32 = roles.into();
        let decoded = Roles::try_from(value).unwrap();
        assert_eq!(decoded, roles);
    }

    #[test]
    fn proto_roundtrip() {
        for role in Role::VARIANTS {
            let proto: ProtoRole = role.into();
            let back: Role = proto.into();
            assert_eq!(back, role);
        }
    }

    #[test]
    fn roles_to_proto_vec() {
        let roles = Roles::from(Role::Owner) + Role::Teacher;
        let vec: Vec<i32> = roles.into();
        assert_eq!(vec.len(), 2);
        assert!(vec.contains(&(ProtoRole::Owner as i32)));
        assert!(vec.contains(&(ProtoRole::Teacher as i32)));
    }

    #[test]
    fn empty_roles_to_proto_vec() {
        let roles = Roles::default();
        let vec: Vec<i32> = roles.into();
        assert!(vec.is_empty());
    }
}
