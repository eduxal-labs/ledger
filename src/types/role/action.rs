use crate::types::error::Error;

type ProtoAction = crate::proto::types::role::Action;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Action {
    Create = 1,
    Read = 2,
    Update = 4,
    Delete = 8,
    Purge = 16,
    Assign = 32,
    Unassign = 64,
    Mark = 128,
    Approve = 256,
}

impl Action {
    pub const VARIANTS: [Self; 9] = [
        Self::Create,
        Self::Read,
        Self::Update,
        Self::Delete,
        Self::Purge,
        Self::Assign,
        Self::Unassign,
        Self::Mark,
        Self::Approve,
    ];
}

impl TryFrom<u16> for Action {
    type Error = Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Action::Create),
            2 => Ok(Action::Read),
            4 => Ok(Action::Update),
            8 => Ok(Action::Delete),
            16 => Ok(Action::Purge),
            32 => Ok(Action::Assign),
            64 => Ok(Action::Unassign),
            128 => Ok(Action::Mark),
            256 => Ok(Action::Approve),
            _ => Err(Error::InvalidAction),
        }
    }
}

impl TryFrom<i32> for Action {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value < 0 || value > u16::MAX as i32 {
            return Err(Error::InvalidAction);
        }
        (value as u16).try_into()
    }
}

impl From<Action> for u16 {
    fn from(action: Action) -> Self {
        action as u16
    }
}

impl From<Action> for i32 {
    fn from(action: Action) -> Self {
        action as u16 as i32
    }
}

impl From<Action> for ProtoAction {
    fn from(action: Action) -> Self {
        match action {
            Action::Create => ProtoAction::Create,
            Action::Read => ProtoAction::Read,
            Action::Update => ProtoAction::Update,
            Action::Delete => ProtoAction::Delete,
            Action::Purge => ProtoAction::Purge,
            Action::Assign => ProtoAction::Assign,
            Action::Unassign => ProtoAction::Unassign,
            Action::Mark => ProtoAction::Mark,
            Action::Approve => ProtoAction::Approve,
        }
    }
}

impl From<ProtoAction> for Action {
    fn from(action: ProtoAction) -> Self {
        match action {
            ProtoAction::Create => Action::Create,
            ProtoAction::Read => Action::Read,
            ProtoAction::Update => Action::Update,
            ProtoAction::Delete => Action::Delete,
            ProtoAction::Purge => Action::Purge,
            ProtoAction::Assign => Action::Assign,
            ProtoAction::Unassign => Action::Unassign,
            ProtoAction::Mark => Action::Mark,
            ProtoAction::Approve => Action::Approve,
        }
    }
}

impl From<super::Actions> for Vec<i32> {
    fn from(actions: super::Actions) -> Self {
        actions
            .iter()
            .map(|a| ProtoAction::from(a) as i32)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmask_values() {
        assert_eq!(Action::Create as u16, 1);
        assert_eq!(Action::Read as u16, 2);
        assert_eq!(Action::Update as u16, 4);
        assert_eq!(Action::Delete as u16, 8);
        assert_eq!(Action::Purge as u16, 16);
        assert_eq!(Action::Assign as u16, 32);
        assert_eq!(Action::Unassign as u16, 64);
        assert_eq!(Action::Mark as u16, 128);
        assert_eq!(Action::Approve as u16, 256);
    }

    #[test]
    fn try_from_u16_valid() {
        for action in Action::VARIANTS {
            let value = action as u16;
            assert_eq!(Action::try_from(value).unwrap(), action);
        }
    }

    #[test]
    fn try_from_u16_invalid() {
        assert!(Action::try_from(0u16).is_err());
        assert!(Action::try_from(3u16).is_err());
        assert!(Action::try_from(512u16).is_err());
    }

    #[test]
    fn try_from_i32_valid() {
        for action in Action::VARIANTS {
            let value = action as u16 as i32;
            assert_eq!(Action::try_from(value).unwrap(), action);
        }
    }

    #[test]
    fn try_from_i32_invalid() {
        assert!(Action::try_from(-1i32).is_err());
        assert!(Action::try_from(0i32).is_err());
        assert!(Action::try_from(3i32).is_err());
        assert!(Action::try_from(70000i32).is_err());
    }

    #[test]
    fn roundtrip_u16() {
        for action in Action::VARIANTS {
            let value: u16 = action.into();
            assert_eq!(Action::try_from(value).unwrap(), action);
        }
    }

    #[test]
    fn roundtrip_i32() {
        for action in Action::VARIANTS {
            let value: i32 = action.into();
            assert_eq!(Action::try_from(value).unwrap(), action);
        }
    }

    #[test]
    fn all_powers_of_two() {
        for action in Action::VARIANTS {
            let v = action as u16;
            assert!(
                v.is_power_of_two(),
                "{:?} = {} is not a power of two",
                action,
                v
            );
        }
    }

    #[test]
    fn variants_count() {
        assert_eq!(Action::VARIANTS.len(), 9);
    }

    #[test]
    fn proto_roundtrip() {
        for action in Action::VARIANTS {
            let proto: ProtoAction = action.into();
            let back: Action = proto.into();
            assert_eq!(back, action);
        }
    }

    #[test]
    fn actions_to_proto_vec() {
        let actions = crate::types::role::Actions::from(Action::Read) + Action::Delete;
        let vec: Vec<i32> = actions.into();
        assert_eq!(vec.len(), 2);
        assert!(vec.contains(&(ProtoAction::Read as i32)));
        assert!(vec.contains(&(ProtoAction::Delete as i32)));
    }

    #[test]
    fn empty_actions_to_proto_vec() {
        let actions = crate::types::role::Actions::default();
        let vec: Vec<i32> = actions.into();
        assert!(vec.is_empty());
    }
}
