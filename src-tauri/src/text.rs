use std::str::FromStr;

use macros::Into;
use thiserror::Error;

#[derive(Clone, Debug, Eq, Hash, Into, PartialEq)]
pub(crate) struct NonEmptyText(String);

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub(crate) enum NonEmptyTextError {
    #[error("text must not be empty")]
    Empty,
    #[error("text must not contain NUL characters")]
    ContainsNul,
}

impl AsRef<str> for NonEmptyText {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl FromStr for NonEmptyText {
    type Err = NonEmptyTextError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(NonEmptyTextError::Empty);
        }
        if value.contains('\0') {
            return Err(NonEmptyTextError::ContainsNul);
        }

        Ok(Self(value.to_owned()))
    }
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
