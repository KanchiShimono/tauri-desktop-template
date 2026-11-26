use super::view::TodoView;
use crate::app::App;
use crate::app::pagination::Page;
use crate::app::port::PortError;
use crate::app::port::todo::Query;

impl<S, C, I> App<S, C, I>
where
    S: Query,
{
    pub(crate) async fn list_active_todos(&self) -> Result<Page<TodoView>, PortError> {
        Ok(self.store.list_active().await?.into())
    }
}

#[cfg(test)]
#[path = "list_active_tests.rs"]
mod tests;
