use domain::todo::Todo;
use domain::todo::TodoByUndeleted;
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
pub(crate) struct UnvalidatedArchiveTodoInput {
    pub(crate) id: Uuid,
}

#[derive(Debug)]
struct ArchiveTodoCommand {
    id: TodoId,
}

#[derive(Debug, Error)]
pub(crate) enum ArchiveTodoError {
    #[error(transparent)]
    NotFound(#[from] TodoNotFound),
    #[error(transparent)]
    Port(#[from] PortError),
}

impl AppError for ArchiveTodoError {
    fn describe(&self) -> ErrorInfo {
        match self {
            Self::NotFound(error) => error.describe(),
            Self::Port(error) => error.describe(),
        }
    }
}

impl From<UnvalidatedArchiveTodoInput> for ArchiveTodoCommand {
    fn from(input: UnvalidatedArchiveTodoInput) -> Self {
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
    pub(crate) async fn archive_todo(
        &self,
        input: UnvalidatedArchiveTodoInput,
    ) -> Result<TodoView, ArchiveTodoError> {
        let command = ArchiveTodoCommand::from(input);
        let now = self.clock.now();

        let todo = self
            .transact(async |uow| archive(uow, command, now).await)
            .await?;
        Ok(TodoView::from(&todo))
    }
}

async fn archive<U>(
    uow: &mut U,
    command: ArchiveTodoCommand,
    now: Timestamp,
) -> Result<Todo, ArchiveTodoError>
where
    U: Repository,
{
    let todo = uow
        .load_undeleted(command.id)
        .await?
        .ok_or(TodoNotFound(command.id))?;
    match TodoByUndeleted::from(todo) {
        TodoByUndeleted::Active(todo) => Ok(uow.save(&todo.archive(now).into_any()).await?),
        TodoByUndeleted::Archived(todo) => Ok(todo.into_any()),
    }
}

#[cfg(test)]
#[path = "archive_tests.rs"]
mod tests;
