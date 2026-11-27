use std::num::TryFromIntError;

use app::error::AppError;
use app::error::ErrorInfo;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("next page offset exceeds the wire format")]
pub(crate) struct NextOffsetOutOfRange(#[from] TryFromIntError);

impl AppError for NextOffsetOutOfRange {
    fn describe(&self) -> ErrorInfo {
        ErrorInfo::Internal
    }
}

#[cfg(test)]
#[path = "pagination_tests.rs"]
mod tests;
