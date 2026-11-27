use thiserror::Error;

use crate::error::AppError;
use crate::error::ErrorInfo;
use crate::error::FieldViolation;
use crate::error::Violations;
use crate::port::PortError;

#[derive(Clone, Copy, Debug)]
pub struct UnvalidatedListQuery {
    pub limit: usize,
    pub offset: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ListQuery {
    limit: usize,
    offset: usize,
}

impl ListQuery {
    pub const MAX_LIMIT: usize = 100;

    pub const fn try_new(limit: usize, offset: usize) -> Result<Self, ListQueryError> {
        if limit == 0 {
            return Err(ListQueryError::ZeroLimit);
        }
        if limit > Self::MAX_LIMIT {
            return Err(ListQueryError::LimitTooLarge {
                limit,
                maximum: Self::MAX_LIMIT,
            });
        }

        Ok(Self { limit, offset })
    }

    pub const fn limit(self) -> usize {
        self.limit
    }

    pub const fn offset(self) -> usize {
        self.offset
    }
}

impl TryFrom<UnvalidatedListQuery> for ListQuery {
    type Error = ListQueryError;

    fn try_from(query: UnvalidatedListQuery) -> Result<Self, Self::Error> {
        Self::try_new(query.limit, query.offset)
    }
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ListQueryError {
    #[error("list limit must be at least 1")]
    ZeroLimit,
    #[error("list limit must not exceed {maximum}, got {limit}")]
    LimitTooLarge { limit: usize, maximum: usize },
}

/// The failures of every paginated list.
#[derive(Debug, Error)]
pub enum ListError {
    #[error(transparent)]
    InvalidQuery(#[from] ListQueryError),
    #[error(transparent)]
    Port(#[from] PortError),
}

impl AppError for ListError {
    fn describe(&self) -> ErrorInfo {
        match self {
            Self::InvalidQuery(error) => {
                let code = match error {
                    ListQueryError::ZeroLimit => "too_small",
                    ListQueryError::LimitTooLarge { .. } => "too_large",
                };
                ErrorInfo::InvalidInput(Violations::one(FieldViolation::new(
                    "limit",
                    code,
                    error.to_string(),
                )))
            }
            Self::Port(error) => error.describe(),
        }
    }
}

/// An ordered list and the offset of its next page, if any.
#[derive(Debug, Eq, PartialEq)]
pub struct Page<T> {
    items: Vec<T>,
    next_offset: Option<usize>,
}

impl<T> Page<T> {
    /// A complete collection, including unpaged collections larger than a page limit.
    pub(crate) fn complete(items: Vec<T>) -> Self {
        Self {
            items,
            next_offset: None,
        }
    }

    /// The query must fetch up to one row beyond its limit to distinguish a full last page.
    pub fn from_lookahead(query: ListQuery, mut rows: Vec<T>) -> Result<Self, NextOffsetOverflow> {
        if rows.len() <= query.limit() {
            return Ok(Self::complete(rows));
        }

        let next_offset = query
            .offset()
            .checked_add(query.limit())
            .ok_or(NextOffsetOverflow {
                offset: query.offset(),
                limit: query.limit(),
            })?;
        rows.truncate(query.limit());
        Ok(Self {
            items: rows,
            next_offset: Some(next_offset),
        })
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn next_offset(&self) -> Option<usize> {
        self.next_offset
    }

    #[must_use]
    pub(crate) fn map<U>(self, f: impl FnMut(T) -> U) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            next_offset: self.next_offset,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("next page offset exceeds usize (offset {offset}, limit {limit})")]
pub struct NextOffsetOverflow {
    offset: usize,
    limit: usize,
}

#[cfg(test)]
#[path = "pagination_tests.rs"]
mod tests;
