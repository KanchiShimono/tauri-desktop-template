use crate::app::App;
use crate::pagination::ListError;
use crate::pagination::ListQuery;
use crate::pagination::Page;
use crate::pagination::UnvalidatedListQuery;
use crate::port::todo::Query;
use crate::todo::view::TodoView;

impl<S, C, I> App<S, C, I>
where
    S: Query,
{
    pub async fn list_archived_todos(
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
