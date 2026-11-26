use super::view::TodoView;
use crate::app::App;
use crate::app::pagination::ListError;
use crate::app::pagination::ListQuery;
use crate::app::pagination::Page;
use crate::app::pagination::UnvalidatedListQuery;
use crate::app::port::todo::Query;

impl<S, C, I> App<S, C, I>
where
    S: Query,
{
    pub(crate) async fn list_archived_todos(
        &self,
        query: UnvalidatedListQuery,
    ) -> Result<Page<TodoView>, ListError> {
        let query = ListQuery::try_from(query)?;
        Ok(self.store.list_archived(query).await?.into())
    }
}

#[cfg(test)]
#[path = "list_archived_tests.rs"]
mod tests;
