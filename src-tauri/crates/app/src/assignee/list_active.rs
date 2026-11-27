use crate::app::App;
use crate::assignee::view::AssigneeView;
use crate::pagination::ListError;
use crate::pagination::ListQuery;
use crate::pagination::Page;
use crate::pagination::UnvalidatedListQuery;
use crate::port::assignee::Query;

impl<S, C, I> App<S, C, I>
where
    S: Query,
{
    pub async fn list_active_assignees(
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
