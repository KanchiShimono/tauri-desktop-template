use crate::app::pagination::ListQuery;

pub(super) struct LimitOffset {
    pub(super) limit: i64,
    pub(super) offset: i64,
}

impl From<ListQuery> for LimitOffset {
    fn from(query: ListQuery) -> Self {
        Self {
            limit: i64::try_from(query.limit() + 1).unwrap_or(i64::MAX),
            offset: i64::try_from(query.offset()).unwrap_or(i64::MAX),
        }
    }
}

#[cfg(test)]
#[path = "pagination_tests.rs"]
mod tests;
