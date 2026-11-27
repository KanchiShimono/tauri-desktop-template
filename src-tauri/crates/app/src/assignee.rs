pub mod create;
pub mod delete;
pub mod get;
mod list_active;
pub mod rename;
pub mod view;

use domain::assignee::AssigneeId;
use domain::assignee::AssigneeNameError;
use thiserror::Error;

use crate::error::AppError;
use crate::error::ErrorInfo;
use crate::error::FieldViolation;
use crate::error::PublicReason;
use crate::error::Violations;

#[derive(Debug, Eq, Error, PartialEq)]
#[error("No active assignee matches {0}.")]
pub struct AssigneeNotFound(pub(crate) AssigneeId);

impl AppError for AssigneeNotFound {
    fn describe(&self) -> ErrorInfo {
        ErrorInfo::NotFound(PublicReason {
            code: "assignee_not_found",
            message: self.to_string(),
        })
    }
}

fn describe_invalid_name(error: &AssigneeNameError) -> ErrorInfo {
    let code = match error {
        AssigneeNameError::Blank => "blank",
        AssigneeNameError::LineBreakOrControlCharacter(_) => "invalid_character",
        AssigneeNameError::TooLong => "too_long",
    };
    ErrorInfo::InvalidInput(Violations::one(FieldViolation::new(
        "name",
        code,
        error.to_string(),
    )))
}

#[cfg(test)]
#[path = "assignee_tests.rs"]
mod tests;
