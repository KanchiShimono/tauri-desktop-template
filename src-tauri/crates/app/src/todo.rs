pub mod archive;
pub mod change_status;
pub mod create;
pub mod delete;
pub mod edit;
pub mod get;
mod list_active;
mod list_archived;
pub mod unarchive;
pub mod view;

use domain::todo::TodoDescriptionError;
use domain::todo::TodoId;
use domain::todo::TodoTitleError;
use thiserror::Error;

use crate::assignee::AssigneeNotFound;
use crate::error::AppError;
use crate::error::ErrorInfo;
use crate::error::FieldViolation;
use crate::error::PublicReason;
use crate::error::Violations;

#[derive(Debug, Eq, PartialEq)]
pub enum TodoFieldError {
    Title(TodoTitleError),
    Description(TodoDescriptionError),
}

impl TodoFieldError {
    fn describe(&self) -> FieldViolation {
        match self {
            Self::Title(error) => {
                let code = match error {
                    TodoTitleError::Blank => "blank",
                    TodoTitleError::LineBreakOrControlCharacter(_) => "invalid_character",
                    TodoTitleError::TooLong => "too_long",
                };
                FieldViolation::new("title", code, error.to_string())
            }
            Self::Description(error) => {
                let code = match error {
                    TodoDescriptionError::Blank => "blank",
                    TodoDescriptionError::InvalidControlCharacter => "invalid_character",
                    TodoDescriptionError::TooLong => "too_long",
                };
                FieldViolation::new("description", code, error.to_string())
            }
        }
    }
}

#[derive(Debug, Eq, Error, PartialEq)]
#[error("No Todo matches {0}.")]
pub struct TodoNotFound(pub TodoId);

impl AppError for TodoNotFound {
    fn describe(&self) -> ErrorInfo {
        ErrorInfo::NotFound(PublicReason {
            code: "todo_not_found",
            message: self.to_string(),
        })
    }
}

#[derive(Debug, Eq, Error, PartialEq)]
#[error("Todo {0} is archived.")]
pub struct TodoArchived(TodoId);

impl AppError for TodoArchived {
    fn describe(&self) -> ErrorInfo {
        ErrorInfo::Conflict(PublicReason {
            code: "todo_archived",
            message: self.to_string(),
        })
    }
}

/// A Todo names its assignee in a field, so an unknown assignee is an input violation.
fn describe_unknown_assignee(error: &AssigneeNotFound) -> ErrorInfo {
    ErrorInfo::InvalidInput(Violations::one(FieldViolation::new(
        "assignee_id",
        "not_found",
        error.to_string(),
    )))
}

#[cfg(test)]
#[path = "todo_tests.rs"]
mod tests;
