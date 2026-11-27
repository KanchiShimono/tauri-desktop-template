use super::view::AssigneeView;
use crate::app::App;
use crate::app::pagination::ListError;
use crate::app::pagination::ListQuery;
use crate::app::pagination::Page;
use crate::app::pagination::UnvalidatedListQuery;
use crate::app::port::assignee::Query;

impl<S, C, I> App<S, C, I>
where
    S: Query,
{
    pub(crate) async fn list_active_assignees(
        &self,
        query: UnvalidatedListQuery,
    ) -> Result<Page<AssigneeView>, ListError> {
        let query = ListQuery::try_from(query)?;
        Ok(self.store.list_active(query).await?.into())
    }
}

#[cfg(test)]
#[path = "list_active_tests.rs"]
mod tests;
