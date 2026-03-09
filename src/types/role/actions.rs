use super::Action;
use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Actions(u16);

const ALL_BITS: u16 = Action::Create as u16
    | Action::Read as u16
    | Action::Update as u16
    | Action::Delete as u16
    | Action::Purge as u16
    | Action::Assign as u16
    | Action::Unassign as u16
    | Action::Mark as u16
    | Action::Approve as u16;

const ALL_ACTIONS: [Action; 9] = [
    Action::Create,
    Action::Read,
    Action::Update,
    Action::Delete,
    Action::Purge,
    Action::Assign,
    Action::Unassign,
    Action::Mark,
    Action::Approve,
];

impl Actions {
    pub fn new(bits: u16) -> Self {
        Self(bits & ALL_BITS)
    }

    pub fn bits(self) -> u16 {
        self.0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    pub fn contains(self, action: Action) -> bool {
        self.0 & action as u16 != 0
    }

    pub fn iter(self) -> impl Iterator<Item = Action> {
        let bits = self.0;
        ALL_ACTIONS
            .into_iter()
            .filter(move |a| bits & *a as u16 != 0)
    }
}

impl From<Action> for Actions {
    fn from(action: Action) -> Self {
        Self(action as u16)
    }
}

impl Add for Actions {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl Add<Action> for Actions {
    type Output = Self;

    fn add(self, rhs: Action) -> Self::Output {
        Self(self.0 | rhs as u16)
    }
}

impl AddAssign for Actions {
    fn add_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl AddAssign<Action> for Actions {
    fn add_assign(&mut self, rhs: Action) {
        self.0 |= rhs as u16;
    }
}

impl Sub for Actions {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 & !rhs.0)
    }
}

impl Sub<Action> for Actions {
    type Output = Self;

    fn sub(self, rhs: Action) -> Self::Output {
        Self(self.0 & !(rhs as u16))
    }
}

impl SubAssign for Actions {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 &= !rhs.0;
    }
}

impl SubAssign<Action> for Actions {
    fn sub_assign(&mut self, rhs: Action) {
        self.0 &= !(rhs as u16);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        let actions = Actions::default();
        assert!(actions.is_empty());
        assert_eq!(actions.len(), 0);
        assert_eq!(actions.bits(), 0);
    }

    #[test]
    fn from_action() {
        let actions = Actions::from(Action::Read);
        assert!(actions.contains(Action::Read));
        assert!(!actions.contains(Action::Create));
        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn add_actions() {
        let a = Actions::from(Action::Create);
        let b = Actions::from(Action::Read);
        let combined = a + b;
        assert!(combined.contains(Action::Create));
        assert!(combined.contains(Action::Read));
        assert_eq!(combined.len(), 2);
    }

    #[test]
    fn add_action() {
        let actions = Actions::default() + Action::Create + Action::Delete;
        assert!(actions.contains(Action::Create));
        assert!(actions.contains(Action::Delete));
        assert!(!actions.contains(Action::Update));
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn add_assign_actions() {
        let mut actions = Actions::from(Action::Create);
        actions += Actions::from(Action::Read);
        assert!(actions.contains(Action::Create));
        assert!(actions.contains(Action::Read));
    }

    #[test]
    fn add_assign_action() {
        let mut actions = Actions::default();
        actions += Action::Assign;
        actions += Action::Unassign;
        assert!(actions.contains(Action::Assign));
        assert!(actions.contains(Action::Unassign));
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn sub_actions() {
        let all = Actions::from(Action::Create) + Action::Read + Action::Update;
        let removed = all - Actions::from(Action::Read);
        assert!(removed.contains(Action::Create));
        assert!(!removed.contains(Action::Read));
        assert!(removed.contains(Action::Update));
    }

    #[test]
    fn sub_action() {
        let actions = Actions::from(Action::Create) + Action::Read;
        let result = actions - Action::Create;
        assert!(!result.contains(Action::Create));
        assert!(result.contains(Action::Read));
    }

    #[test]
    fn sub_assign_actions() {
        let mut actions = Actions::from(Action::Create) + Action::Read + Action::Delete;
        actions -= Actions::from(Action::Read);
        assert!(!actions.contains(Action::Read));
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn sub_assign_action() {
        let mut actions = Actions::from(Action::Mark) + Action::Approve;
        actions -= Action::Mark;
        assert!(!actions.contains(Action::Mark));
        assert!(actions.contains(Action::Approve));
    }

    #[test]
    fn idempotent_add() {
        let actions = Actions::from(Action::Create) + Action::Create;
        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn sub_absent_is_noop() {
        let actions = Actions::from(Action::Read);
        let result = actions - Action::Delete;
        assert_eq!(result, actions);
    }

    #[test]
    fn iter_all() {
        let mut actions = Actions::default();
        for action in ALL_ACTIONS {
            actions += action;
        }
        let collected: Vec<Action> = actions.iter().collect();
        assert_eq!(collected.len(), 9);
        assert_eq!(collected, ALL_ACTIONS.to_vec());
    }

    #[test]
    fn iter_subset() {
        let actions = Actions::from(Action::Purge) + Action::Approve;
        let collected: Vec<Action> = actions.iter().collect();
        assert_eq!(collected, vec![Action::Purge, Action::Approve]);
    }

    #[test]
    fn new_masks_invalid_bits() {
        let actions = Actions::new(0xFFFF);
        assert_eq!(actions.bits(), ALL_BITS);
        assert_eq!(actions.len(), 9);
    }

    #[test]
    fn contains_all_nine_actions() {
        let mut actions = Actions::default();
        actions += Action::Create;
        actions += Action::Read;
        actions += Action::Update;
        actions += Action::Delete;
        actions += Action::Purge;
        actions += Action::Assign;
        actions += Action::Unassign;
        actions += Action::Mark;
        actions += Action::Approve;
        assert_eq!(actions.len(), 9);
        assert!(!actions.is_empty());
        for action in ALL_ACTIONS {
            assert!(actions.contains(action));
        }
    }
}
