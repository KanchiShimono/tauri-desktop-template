use domain::todo::TodoId;
use thiserror::Error;
use uuid::Uuid;

use crate::app::App;
use crate::error::AppError;
use crate::error::ErrorInfo;
use crate::port::PortError;
use crate::port::todo::Query;
use crate::todo::TodoNotFound;
use crate::todo::view::TodoView;

#[derive(Debug, Error)]
pub enum GetTodoError {
    #[error(transparent)]
    NotFound(#[from] TodoNotFound),
    #[error(transparent)]
    Port(#[from] PortError),
}

impl AppError for GetTodoError {
    fn describe(&self) -> ErrorInfo {
        match self {
            Self::NotFound(error) => error.describe(),
            Self::Port(error) => error.describe(),
        }
    }
}

impl<S, C, I> App<S, C, I>
where
    S: Query,
{
    pub async fn get_todo(&self, id: Uuid) -> Result<TodoView, GetTodoError> {
        let id = TodoId::from(id);
        let todo = self.store.find(id).await?.ok_or(TodoNotFound(id))?;
        Ok(TodoView::from(&todo))
    }
}

#[cfg(test)]
#[path = "get_tests.rs"]
mod tests;
