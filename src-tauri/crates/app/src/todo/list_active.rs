use crate::app::App;
use crate::pagination::Page;
use crate::port::PortError;
use crate::port::todo::Query;
use crate::todo::view::TodoView;

impl<S, C, I> App<S, C, I>
where
    S: Query,
{
    pub async fn list_active_todos(&self) -> Result<Page<TodoView>, PortError> {
        Ok(self.store.list_active().await?.into())
    }
}

#[cfg(test)]
#[path = "list_active_tests.rs"]
mod tests;
