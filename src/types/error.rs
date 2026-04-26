#![allow(unused)]
use crossbeam::channel::TrySendError;
use diesel::result::{DatabaseErrorKind as ErrorKind, Error as DieselError};
use std::fmt::Display;
use tonic::Status;
use tracing::error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub trait OnConflict: Sized {
    type Error: Conflict;
    type Resolved;
    fn on_conflict(self, err: Self::Error) -> Self;
    fn resolve(self) -> Self::Resolved;
}

pub trait Conflict: Sized {
    fn conflict(&self) -> bool;
}

pub trait ForeignKeyError: Sized {
    fn foreign_key(&self) -> bool;
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid id")]
    InvalidId,
    #[error("invalid phone")]
    InvalidPhone,
    #[error("invalid date")]
    InvalidDate,
    #[error("invalid time")]
    InvalidTime,
    #[error("invalid verification code")]
    InvalidVerificationCode,
    #[error("invalid status")]
    InvalidStatus,
    #[error("invalid level")]
    InvalidLevel,
    #[error("invalid role")]
    InvalidRole,
    #[error("unauthorized")]
    Unauthorized,
    #[error("invalid token")]
    InvalidToken,
    #[error("user not found")]
    UserNotFound,
    #[error("role not found")]
    RoleNotFound,
    #[error("role already exists")]
    RoleAlreadyExists,
    #[error("user already exists")]
    UserAlreadyExists,
    #[error("invalid resource")]
    InvalidResource,
    #[error("invalid action")]
    InvalidAction,
    #[error("invalid permissions")]
    InvalidPermissions,
    #[error("invalid question text")]
    InvalidQuestionText,
    #[error("invalid question marks")]
    InvalidQuestionMarks,
    #[error("invalid rubric marks")]
    InvalidRubricMarks,
    #[error("invalid bulk import json")]
    InvalidBulkImportJson,
    #[error("invalid curriculum: expected \"844\" or \"cbc\"")]
    InvalidCurriculum,
    #[error("question already exists")]
    QuestionAlreadyExists,
    #[error("school not found")]
    SchoolNotFound,
    #[error("subject not found")]
    SubjectNotFound,
    #[error("topic not found")]
    TopicNotFound,
    #[error("invalid county")]
    InvalidCounty,
    #[error("forbidden")]
    Forbidden,
    #[error("conflict")]
    Conflict,
    #[error("foreign key constraint violated")]
    ForeignKey,
    #[error("database is busy, try again")]
    DatabaseLocked,
    #[error("slow down")]
    SlowDown,
    #[error("nothing to update")]
    NothingToUpdate,
    #[error("not enough questions")]
    NotEnoughQuestions,
    #[error("not found")]
    NotFound,
    #[error("event not found")]
    EventNotFound,
    #[error("paper not found")]
    PaperNotFound,
    #[error("paper schedule not found")]
    PaperScheduleNotFound,
    #[error("paper already finalized")]
    PaperAlreadyFinalized,
    #[error("paper not yet revealed")]
    PaperNotRevealed,
    #[error("not enough questions for topic allocation")]
    NotEnoughQuestionsForAllocation,
    #[error("generation in progress")]
    GenerationInProgress,
    #[error("invalid paper status transition")]
    InvalidStatusTransition,
    #[error("coverage not confirmed")]
    CoverageNotConfirmed,
    #[error("internal server error")]
    Internal,
}

impl From<DieselError> for Error {
    fn from(err: DieselError) -> Self {
        match &err {
            DieselError::DatabaseError(kind, info) => match kind {
                ErrorKind::UniqueViolation => return Error::Conflict,
                ErrorKind::ForeignKeyViolation => {
                    error!("foreign key violation: {}", info.message());
                    return Error::ForeignKey;
                }
                // SQLite reports SQLITE_BUSY / SQLITE_LOCKED as Unknown
                // with messages containing "database is locked" or "database table is locked".
                ErrorKind::Unknown => {
                    let msg = info.message();
                    if msg.contains("locked") || msg.contains("busy") {
                        error!("database locked: {msg}");
                        return Error::DatabaseLocked;
                    }
                }
                _ => {}
            },
            _ => {}
        }
        error!("{}", err);
        Error::Internal
    }
}

impl<T> From<TrySendError<T>> for Error {
    fn from(err: TrySendError<T>) -> Self {
        match err {
            TrySendError::Full(_) => Error::SlowDown,
            TrySendError::Disconnected(_) => Error::Internal,
        }
    }
}

impl Error {
    pub fn internal<E: Display>(err: E) -> Self {
        error!("{}", err);
        Self::Internal
    }

    pub fn invalid_token<E: Display>(_: E) -> Self {
        Self::InvalidToken
    }

    pub fn invalid_code<E: Display>(_: E) -> Self {
        Self::InvalidVerificationCode
    }
}

impl From<Error> for Status {
    fn from(err: Error) -> Self {
        match err {
            Error::InvalidId => Status::invalid_argument("invalid id"),
            Error::InvalidPhone => Status::invalid_argument("invalid phone"),
            Error::InvalidDate => Status::invalid_argument("invalid date"),
            Error::InvalidTime => Status::invalid_argument("invalid time"),
            Error::InvalidVerificationCode => Status::invalid_argument("invalid verification code"),
            Error::InvalidStatus => Status::invalid_argument("invalid status"),
            Error::InvalidLevel => Status::invalid_argument("invalid level"),
            Error::InvalidRole => Status::invalid_argument("invalid role"),
            Error::Unauthorized => Status::unauthenticated("unauthorized"),
            Error::InvalidToken => Status::unauthenticated("invalid token"),
            Error::UserNotFound => Status::not_found("user not found"),
            Error::RoleNotFound => Status::not_found("role not found"),
            Error::RoleAlreadyExists => Status::already_exists("role already exists"),
            Error::UserAlreadyExists => Status::already_exists("user already exists"),
            Error::QuestionAlreadyExists => Status::already_exists("question already exists"),
            Error::InvalidResource => Status::invalid_argument("invalid resource"),
            Error::InvalidAction => Status::invalid_argument("invalid action"),
            Error::InvalidPermissions => Status::invalid_argument("invalid permissions"),
            Error::InvalidQuestionText => Status::invalid_argument("invalid question text"),
            Error::InvalidQuestionMarks => Status::invalid_argument("invalid question marks"),
            Error::InvalidRubricMarks => Status::invalid_argument("invalid rubric marks"),
            Error::InvalidBulkImportJson => Status::invalid_argument("invalid bulk import json"),
            Error::InvalidCurriculum => {
                Status::invalid_argument("invalid curriculum: expected \"844\" or \"cbc\"")
            }
            Error::SchoolNotFound => Status::not_found("school not found"),
            Error::SubjectNotFound => Status::not_found("subject not found"),
            Error::TopicNotFound => Status::not_found("topic not found"),
            Error::InvalidCounty => Status::invalid_argument("invalid county"),
            Error::Forbidden => Status::permission_denied("permission denied"),
            Error::Conflict => Status::already_exists("record already exists"),
            Error::ForeignKey => Status::failed_precondition("referenced record does not exist"),
            Error::DatabaseLocked => Status::unavailable("database is busy, try again"),
            Error::SlowDown => Status::resource_exhausted("please try again after a few minutes"),
            Error::NothingToUpdate => Status::failed_precondition("nothing to update"),
            Error::NotEnoughQuestions => Status::failed_precondition(
                "not enough questions in the bank for this topic and mark allocation",
            ),
            Error::NotFound => Status::not_found("not found"),
            Error::EventNotFound => Status::not_found("event not found"),
            Error::PaperNotFound => Status::not_found("paper not found"),
            Error::PaperScheduleNotFound => Status::not_found("paper schedule not found"),
            Error::PaperAlreadyFinalized => Status::failed_precondition("paper already finalized"),
            Error::PaperNotRevealed => {
                Status::failed_precondition("paper questions not yet revealed")
            }
            Error::NotEnoughQuestionsForAllocation => Status::failed_precondition(
                "not enough questions in the bank for this topic/mark allocation",
            ),
            Error::GenerationInProgress => {
                Status::failed_precondition("generation already in progress")
            }
            Error::InvalidStatusTransition => {
                Status::failed_precondition("invalid paper status transition")
            }
            Error::CoverageNotConfirmed => {
                Status::failed_precondition("exam coverage not confirmed by admin")
            }
            Error::Internal => Status::internal("internal server error"),
        }
    }
}

impl Conflict for Error {
    fn conflict(&self) -> bool {
        matches!(self, Self::Conflict)
    }
}

impl ForeignKeyError for Error {
    fn foreign_key(&self) -> bool {
        matches!(self, Self::ForeignKey)
    }
}

impl<Ok, Error: Conflict> OnConflict for std::result::Result<Ok, Error> {
    type Error = Error;
    type Resolved = std::result::Result<(), Self::Error>;

    fn on_conflict(self, err: Self::Error) -> Self {
        match self {
            Ok(ok) => Ok(ok),
            Err(error) => match error.conflict() {
                true => Err(err),
                false => Err(error),
            },
        }
    }

    fn resolve(self) -> Self::Resolved {
        match self {
            Ok(_) => Ok(()),
            Err(err) => match err.conflict() {
                true => Ok(()),
                false => Err(err),
            },
        }
    }
}
