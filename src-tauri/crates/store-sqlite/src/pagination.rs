use app::pagination::ListQuery;

pub(crate) struct LimitOffset {
    pub(crate) limit: i64,
    pub(crate) offset: i64,
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
