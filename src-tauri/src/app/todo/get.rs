use domain::todo::TodoId;
use thiserror::Error;
use uuid::Uuid;

use super::TodoNotFound;
use super::view::TodoView;
use crate::app::App;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::port::PortError;
use crate::app::port::todo::Query;

#[derive(Debug, Error)]
pub(crate) enum GetTodoError {
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
    pub(crate) async fn get_todo(&self, id: Uuid) -> Result<TodoView, GetTodoError> {
        let id = TodoId::from(id);
        let todo = self.store.find(id).await?.ok_or(TodoNotFound(id))?;
        Ok(TodoView::from(&todo))
    }
}

#[cfg(test)]
#[path = "get_tests.rs"]
mod tests;
