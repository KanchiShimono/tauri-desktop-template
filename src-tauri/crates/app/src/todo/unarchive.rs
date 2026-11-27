use domain::todo::Todo;
use domain::todo::TodoByUndeleted;
use domain::todo::TodoId;
use thiserror::Error;
use uuid::Uuid;

use crate::app::App;
use crate::error::AppError;
use crate::error::ErrorInfo;
use crate::port::PortError;
use crate::port::UnitOfWorkFactory;
use crate::port::todo::Repository;
use crate::todo::TodoNotFound;
use crate::todo::view::TodoView;

#[derive(Clone, Debug)]
pub struct UnvalidatedUnarchiveTodoInput {
    pub id: Uuid,
}

#[derive(Debug)]
struct UnarchiveTodoCommand {
    id: TodoId,
}

#[derive(Debug, Error)]
pub enum UnarchiveTodoError {
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
    pub async fn unarchive_todo(
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
