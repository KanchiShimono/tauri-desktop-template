use thiserror::Error;
use uuid::Uuid;

use super::TodoNotFound;
use super::view::TodoView;
use crate::app::App;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::port::PortError;
use crate::app::port::UnitOfWorkFactory;
use crate::app::port::todo::Repository;
use crate::domain::todo::Todo;
use crate::domain::todo::TodoByUndeleted;
use crate::domain::todo::TodoId;

#[derive(Clone, Debug)]
pub(crate) struct UnvalidatedUnarchiveTodoInput {
    pub(crate) id: Uuid,
}

#[derive(Debug)]
struct UnarchiveTodoCommand {
    id: TodoId,
}

#[derive(Debug, Error)]
pub(crate) enum UnarchiveTodoError {
    #[error(transparent)]
    NotFound(#[from] TodoNotFound),
    #[error(transparent)]
    Port(#[from] PortError),
}

impl AppError for UnarchiveTodoError {
    fn describe(&self) -> ErrorInfo {
        match self {
            Self::NotFound(error) => error.describe(),
            Self::Port(error) => error.describe(),
        }
    }
}

impl From<UnvalidatedUnarchiveTodoInput> for UnarchiveTodoCommand {
    fn from(input: UnvalidatedUnarchiveTodoInput) -> Self {
        Self {
            id: input.id.into(),
        }
    }
}

impl<S, C, I> App<S, C, I>
where
    S: UnitOfWorkFactory,
    S::UnitOfWork: Repository,
{
    pub(crate) async fn unarchive_todo(
        &self,
        input: UnvalidatedUnarchiveTodoInput,
    ) -> Result<TodoView, UnarchiveTodoError> {
        let command = UnarchiveTodoCommand::from(input);

        let todo = self
            .transact(async |uow| unarchive(uow, command).await)
            .await?;
        Ok(TodoView::from(&todo))
    }
}

async fn unarchive<U>(
    uow: &mut U,
    command: UnarchiveTodoCommand,
) -> Result<Todo, UnarchiveTodoError>
where
    U: Repository,
{
    let todo = uow
        .load_undeleted(command.id)
        .await?
        .ok_or(TodoNotFound(command.id))?;
    match TodoByUndeleted::from(todo) {
        TodoByUndeleted::Active(todo) => Ok(todo.into_any()),
        TodoByUndeleted::Archived(todo) => Ok(uow.save(&todo.unarchive().into_any()).await?),
    }
}

#[cfg(test)]
#[path = "unarchive_tests.rs"]
mod tests;
