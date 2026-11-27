use domain::todo::Todo;
use domain::todo::TodoId;
use jiff::Timestamp;
use thiserror::Error;
use uuid::Uuid;

use super::TodoNotFound;
use super::view::TodoView;
use crate::app::App;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::port::Clock;
use crate::app::port::PortError;
use crate::app::port::UnitOfWorkFactory;
use crate::app::port::todo::Repository;

#[derive(Clone, Debug)]
pub(crate) struct UnvalidatedDeleteTodoInput {
    pub(crate) id: Uuid,
}

#[derive(Debug)]
struct DeleteTodoCommand {
    id: TodoId,
}

#[derive(Debug, Error)]
pub(crate) enum DeleteTodoError {
    #[error(transparent)]
    NotFound(#[from] TodoNotFound),
    #[error(transparent)]
    Port(#[from] PortError),
}

impl AppError for DeleteTodoError {
    fn describe(&self) -> ErrorInfo {
        match self {
            Self::NotFound(error) => error.describe(),
            Self::Port(error) => error.describe(),
        }
    }
}

impl From<UnvalidatedDeleteTodoInput> for DeleteTodoCommand {
    fn from(input: UnvalidatedDeleteTodoInput) -> Self {
        Self {
            id: input.id.into(),
        }
    }
}

impl<S, C, I> App<S, C, I>
where
    S: UnitOfWorkFactory,
    S::UnitOfWork: Repository,
    C: Clock,
{
    pub(crate) async fn delete_todo(
        &self,
        input: UnvalidatedDeleteTodoInput,
    ) -> Result<TodoView, DeleteTodoError> {
        let command = DeleteTodoCommand::from(input);
        let now = self.clock.now();

        let todo = self
            .transact(async |uow| delete(uow, command, now).await)
            .await?;
        Ok(TodoView::from(&todo))
    }
}

async fn delete<U>(
    uow: &mut U,
    command: DeleteTodoCommand,
    now: Timestamp,
) -> Result<Todo, DeleteTodoError>
where
    U: Repository,
{
    let todo = uow
        .load_undeleted(command.id)
        .await?
        .ok_or(TodoNotFound(command.id))?;
    Ok(uow.save(&todo.delete(now).into_any()).await?)
}

#[cfg(test)]
#[path = "delete_tests.rs"]
mod tests;
