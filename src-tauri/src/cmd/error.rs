use std::error::Error;
use std::iter;

use app::error::AppError;
use app::error::ErrorInfo;
use app::error::FieldViolation;
use serde::Serialize;
use specta::Type;

#[derive(Debug, Eq, PartialEq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum CommandError {
    InvalidInput {
        code: &'static str,
        message: String,
        violations: Vec<Violation>,
    },
    NotFound {
        code: &'static str,
        message: String,
    },
    Conflict {
        code: &'static str,
        message: String,
    },
    Unavailable {
        code: &'static str,
        message: String,
    },
    Internal {
        code: &'static str,
        message: String,
    },
}

#[derive(Debug, Eq, PartialEq, Serialize, Type)]
pub(crate) struct Violation {
    field: &'static str,
    code: &'static str,
    message: String,
}

impl From<&FieldViolation> for Violation {
    fn from(violation: &FieldViolation) -> Self {
        Self {
            field: violation.field,
            code: violation.reason.code,
            message: violation.reason.message.clone(),
        }
    }
}

impl From<&ErrorInfo> for CommandError {
    fn from(info: &ErrorInfo) -> Self {
        let code = info.code();
        let message = info.message().to_owned();
        match info {
            ErrorInfo::InvalidInput(violations) => Self::InvalidInput {
                code,
                message,
                violations: iter::once(&violations.first)
                    .chain(&violations.rest)
                    .map(Into::into)
                    .collect(),
            },
            ErrorInfo::NotFound(_) => Self::NotFound { code, message },
            ErrorInfo::Conflict(_) => Self::Conflict { code, message },
            ErrorInfo::Unavailable => Self::Unavailable { code, message },
            ErrorInfo::Internal => Self::Internal { code, message },
        }
    }
}

pub(crate) fn report<E: AppError>(error: E) -> CommandError {
    let info = error.describe();
    match info {
        ErrorInfo::Internal => {
            tracing::error!(error = &error as &dyn Error, "command failed");
        }
        ErrorInfo::Unavailable => {
            tracing::warn!(error = &error as &dyn Error, "command unavailable");
        }
        ErrorInfo::InvalidInput(_) | ErrorInfo::NotFound(_) | ErrorInfo::Conflict(_) => {}
    }
    (&info).into()
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
