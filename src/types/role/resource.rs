use crate::types::error::Error;

type ProtoResource = crate::proto::types::role::Resource;

#[derive(Debug, macros::Count, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resource {
    Users = 1,
    Schools = 2,
    Owners = 3,
    Teachers = 4,
    Staff = 5,
    Students = 6,
    Departments = 7,
    Classes = 8,
    Attendance = 9,
    Lessons = 10,
    Exams = 11,
    Grades = 12,
    Fees = 13,
    Payments = 14,
    Announcements = 15,
    Roles = 16,
    Plans = 17,
    AI = 18,
    Subjects = 19,
}

impl TryFrom<u8> for Resource {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Resource::Users),
            2 => Ok(Resource::Schools),
            3 => Ok(Resource::Owners),
            4 => Ok(Resource::Teachers),
            5 => Ok(Resource::Staff),
            6 => Ok(Resource::Students),
            7 => Ok(Resource::Departments),
            8 => Ok(Resource::Classes),
            9 => Ok(Resource::Attendance),
            10 => Ok(Resource::Lessons),
            11 => Ok(Resource::Exams),
            12 => Ok(Resource::Grades),
            13 => Ok(Resource::Fees),
            14 => Ok(Resource::Payments),
            15 => Ok(Resource::Announcements),
            16 => Ok(Resource::Roles),
            17 => Ok(Resource::Plans),
            18 => Ok(Resource::AI),
            19 => Ok(Resource::Subjects),
            _ => Err(Error::InvalidResource),
        }
    }
}

impl TryFrom<i32> for Resource {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value < 0 || value > u8::MAX as i32 {
            return Err(Error::InvalidResource);
        }
        Resource::try_from(value as u8)
    }
}

impl From<Resource> for u8 {
    fn from(resource: Resource) -> Self {
        resource as u8
    }
}

impl From<Resource> for usize {
    fn from(resource: Resource) -> Self {
        (resource as u8 - 1) as usize
    }
}

impl From<Resource> for ProtoResource {
    fn from(resource: Resource) -> Self {
        match resource {
            Resource::Users => ProtoResource::Users,
            Resource::Schools => ProtoResource::Schools,
            Resource::Owners => ProtoResource::Owners,
            Resource::Teachers => ProtoResource::Teachers,
            Resource::Staff => ProtoResource::Staff,
            Resource::Students => ProtoResource::Students,
            Resource::Departments => ProtoResource::Departments,
            Resource::Classes => ProtoResource::Classes,
            Resource::Attendance => ProtoResource::Attendance,
            Resource::Lessons => ProtoResource::Lessons,
            Resource::Exams => ProtoResource::Exams,
            Resource::Grades => ProtoResource::Grades,
            Resource::Fees => ProtoResource::Fees,
            Resource::Payments => ProtoResource::Payments,
            Resource::Announcements => ProtoResource::Announcements,
            Resource::Roles => ProtoResource::Roles,
            Resource::Plans => ProtoResource::Plans,
            Resource::AI => ProtoResource::Ai,
            Resource::Subjects => ProtoResource::Subjects,
        }
    }
}

impl From<ProtoResource> for Resource {
    fn from(resource: ProtoResource) -> Self {
        match resource {
            ProtoResource::Users => Resource::Users,
            ProtoResource::Schools => Resource::Schools,
            ProtoResource::Owners => Resource::Owners,
            ProtoResource::Teachers => Resource::Teachers,
            ProtoResource::Staff => Resource::Staff,
            ProtoResource::Students => Resource::Students,
            ProtoResource::Departments => Resource::Departments,
            ProtoResource::Classes => Resource::Classes,
            ProtoResource::Attendance => Resource::Attendance,
            ProtoResource::Lessons => Resource::Lessons,
            ProtoResource::Exams => Resource::Exams,
            ProtoResource::Grades => Resource::Grades,
            ProtoResource::Fees => Resource::Fees,
            ProtoResource::Payments => Resource::Payments,
            ProtoResource::Announcements => Resource::Announcements,
            ProtoResource::Roles => Resource::Roles,
            ProtoResource::Plans => Resource::Plans,
            ProtoResource::Ai => Resource::AI,
            ProtoResource::Subjects => Resource::Subjects,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count() {
        assert_eq!(Resource::COUNT, 19);
    }

    #[test]
    fn variants() {
        assert_eq!(Resource::VARIANTS.len(), 19);
        assert_eq!(Resource::VARIANTS[0], Resource::Users);
        assert_eq!(Resource::VARIANTS[17], Resource::AI);
        assert_eq!(Resource::VARIANTS[18], Resource::Subjects);
    }

    #[test]
    fn try_from_u8() {
        assert_eq!(Resource::try_from(1u8).unwrap(), Resource::Users);
        assert_eq!(Resource::try_from(9u8).unwrap(), Resource::Attendance);
        assert_eq!(Resource::try_from(18u8).unwrap(), Resource::AI);
        assert_eq!(Resource::try_from(19u8).unwrap(), Resource::Subjects);
        assert!(Resource::try_from(0u8).is_err());
        assert!(Resource::try_from(20u8).is_err());
    }

    #[test]
    fn try_from_i32() {
        assert_eq!(Resource::try_from(1i32).unwrap(), Resource::Users);
        assert_eq!(Resource::try_from(18i32).unwrap(), Resource::AI);
        assert_eq!(Resource::try_from(19i32).unwrap(), Resource::Subjects);
        assert!(Resource::try_from(0i32).is_err());
        assert!(Resource::try_from(-1i32).is_err());
        assert!(Resource::try_from(20i32).is_err());
    }

    #[test]
    fn into_u8() {
        assert_eq!(u8::from(Resource::Users), 1);
        assert_eq!(u8::from(Resource::AI), 18);
        assert_eq!(u8::from(Resource::Subjects), 19);
    }

    #[test]
    fn into_usize() {
        assert_eq!(usize::from(Resource::Users), 0);
        assert_eq!(usize::from(Resource::Schools), 1);
        assert_eq!(usize::from(Resource::AI), 17);
        assert_eq!(usize::from(Resource::Subjects), 18);
    }

    #[test]
    fn roundtrip_u8() {
        for resource in Resource::VARIANTS {
            let byte = u8::from(resource);
            assert_eq!(Resource::try_from(byte).unwrap(), resource);
        }
    }

    #[test]
    fn proto_roundtrip() {
        for resource in Resource::VARIANTS {
            let proto: ProtoResource = resource.into();
            let back: Resource = proto.into();
            assert_eq!(back, resource);
        }
    }

    #[test]
    fn proto_try_from_i32() {
        for resource in Resource::VARIANTS {
            let proto: ProtoResource = resource.into();
            let i = proto as i32;
            let back = Resource::try_from(ProtoResource::try_from(i).unwrap()).unwrap();
            assert_eq!(back, resource);
        }
    }
}
