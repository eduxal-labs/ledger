#![allow(dead_code)]

use crate::types::error::Error;
use crate::types::id::Id;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Organisation {
    System,
    Account,
    School(Id),
}

impl Organisation {
    pub fn optional(&self) -> Option<Id> {
        match self {
            Organisation::School(id) => Some(*id),
            _ => None,
        }
    }
}

impl From<Id> for Organisation {
    fn from(id: Id) -> Self {
        Organisation::School(id)
    }
}

impl From<Option<Id>> for Organisation {
    fn from(id: Option<Id>) -> Self {
        match id {
            Some(id) => Organisation::School(id),
            None => Organisation::System,
        }
    }
}

impl PartialEq<Id> for Organisation {
    fn eq(&self, other: &Id) -> bool {
        match self {
            Organisation::School(id) => id == other,
            _ => false,
        }
    }
}

impl FromStr for Organisation {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "system" => Ok(Organisation::System),
            "account" => Ok(Organisation::Account),
            id => Ok(Organisation::School(id.parse()?)),
        }
    }
}

impl TryFrom<String> for Organisation {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_id() {
        let id = Id::default();
        let org = Organisation::from(id);
        assert_eq!(org, Organisation::School(id));
    }

    #[test]
    fn from_some_id() {
        let id = Id::default();
        let org = Organisation::from(Some(id));
        assert_eq!(org, Organisation::School(id));
    }

    #[test]
    fn from_none() {
        let org = Organisation::from(None);
        assert_eq!(org, Organisation::System);
    }

    #[test]
    fn optional_school() {
        let id = Id::default();
        let org = Organisation::School(id);
        assert_eq!(org.optional(), Some(id));
    }

    #[test]
    fn optional_system() {
        assert_eq!(Organisation::System.optional(), None);
    }

    #[test]
    fn optional_account() {
        assert_eq!(Organisation::Account.optional(), None);
    }

    #[test]
    fn partial_eq_id_match() {
        let id = Id::default();
        let org = Organisation::School(id);
        assert_eq!(org, id);
    }

    #[test]
    fn partial_eq_id_no_match() {
        let id1 = Id::default();
        let id2 = Id::default();
        let org = Organisation::School(id1);
        assert_ne!(org, id2);
    }

    #[test]
    fn partial_eq_id_system() {
        let id = Id::default();
        assert_ne!(Organisation::System, id);
    }

    #[test]
    fn from_str_system() {
        let org: Organisation = "system".parse().unwrap();
        assert_eq!(org, Organisation::System);
    }

    #[test]
    fn from_str_account() {
        let org: Organisation = "account".parse().unwrap();
        assert_eq!(org, Organisation::Account);
    }

    #[test]
    fn from_str_school_id() {
        let id = Id::default();
        let org: Organisation = id.to_string().parse().unwrap();
        assert_eq!(org, Organisation::School(id));
    }

    #[test]
    fn from_str_invalid() {
        let result: Result<Organisation, _> = "not-a-valid-id".parse();
        assert!(result.is_err());
    }

    #[test]
    fn try_from_string() {
        let org = Organisation::try_from("system".to_string()).unwrap();
        assert_eq!(org, Organisation::System);
    }
}
