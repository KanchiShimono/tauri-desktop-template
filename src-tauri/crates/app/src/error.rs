use std::error::Error;

use crate::port::PortError;

pub trait AppError: Error + Send + Sync + 'static {
    /// Describes the failure using only information suitable for callers.
    fn describe(&self) -> ErrorInfo;
}

#[derive(Debug, Eq, PartialEq)]
pub enum ErrorInfo {
    InvalidInput(Violations<FieldViolation>),
    NotFound(PublicReason),
    Conflict(PublicReason),
    Unavailable,
    Internal,
}

impl ErrorInfo {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "invalid_input",
            Self::NotFound(reason) | Self::Conflict(reason) => reason.code,
            Self::Unavailable => "temporarily_unavailable",
            Self::Internal => "internal",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::InvalidInput(_) => "The input is invalid.",
            Self::NotFound(reason) | Self::Conflict(reason) => &reason.message,
            Self::Unavailable => "The service is temporarily unavailable.",
            Self::Internal => "An internal error occurred.",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Violations<T> {
    pub first: T,
    pub rest: Vec<T>,
}

impl<T> Violations<T> {
    pub(crate) const fn one(first: T) -> Self {
        Self {
            first,
            rest: Vec::new(),
        }
    }

    pub(crate) fn map_ref<U>(&self, mut f: impl FnMut(&T) -> U) -> Violations<U> {
        Violations {
            first: f(&self.first),
            rest: self.rest.iter().map(f).collect(),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct FieldViolation {
    pub field: &'static str,
    pub reason: PublicReason,
}

impl FieldViolation {
    pub fn new(field: &'static str, code: &'static str, message: String) -> Self {
        Self {
            field,
            reason: PublicReason { code, message },
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct PublicReason {
    /// Stable, lower_snake_case identifier for programmatic use.
    pub code: &'static str,
    pub message: String,
}

impl AppError for PortError {
    fn describe(&self) -> ErrorInfo {
        match self {
            Self::Busy(_) => ErrorInfo::Unavailable,
            Self::Backend(_) => ErrorInfo::Internal,
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
