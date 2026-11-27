use domain::todo::StatusKind;
use domain::todo::Todo;
use domain::todo::TodoByUndeleted;
use domain::todo::TodoId;
use jiff::Timestamp;
use thiserror::Error;
use uuid::Uuid;

use crate::app::App;
use crate::error::AppError;
use crate::error::ErrorInfo;
use crate::port::Clock;
use crate::port::PortError;
use crate::port::UnitOfWorkFactory;
use crate::port::todo::Repository;
use crate::todo::TodoArchived;
use crate::todo::TodoNotFound;
use crate::todo::view::TodoView;

#[derive(Clone, Debug)]
pub struct UnvalidatedChangeTodoStatusInput {
    pub id: Uuid,
    pub status: TodoStatusInput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TodoStatusInput {
    NotStarted,
    InProgress,
    Done,
}

#[derive(Debug)]
struct ChangeTodoStatusCommand {
    id: TodoId,
    status: StatusKind,
}

#[derive(Debug, Error)]
pub enum ChangeTodoStatusError {
    #[error(transparent)]
    NotFound(#[from] TodoNotFound),
    #[error(transparent)]
    Archived(#[from] TodoArchived),
    #[error(transparent)]
    Port(#[from] PortError),
}

impl AppError for ChangeTodoStatusError {
    fn describe(&self) -> ErrorInfo {
        match self {
            Self::NotFound(error) => error.describe(),
            Self::Archived(error) => error.describe(),
            Self::Port(error) => error.describe(),
        }
    }
}

impl From<TodoStatusInput> for StatusKind {
    fn from(status: TodoStatusInput) -> Self {
        match status {
            TodoStatusInput::NotStarted => Self::NotStarted,
            TodoStatusInput::InProgress => Self::InProgress,
            TodoStatusInput::Done => Self::Done,
        }
    }
}

impl From<UnvalidatedChangeTodoStatusInput> for ChangeTodoStatusCommand {
    fn from(input: UnvalidatedChangeTodoStatusInput) -> Self {
        Self {
            id: input.id.into(),
            status: input.status.into(),
        }
    }
}

impl<S, C, I> App<S, C, I>
where
    S: UnitOfWorkFactory,
    S::UnitOfWork: Repository,
    C: Clock,
{
    pub async fn change_todo_status(
        &self,
        input: UnvalidatedChangeTodoStatusInput,
    ) -> Result<TodoView, ChangeTodoStatusError> {
        let command = ChangeTodoStatusCommand::from(input);
        let now = self.clock.now();

        let todo = self
            .transact(async |uow| change_status(uow, command, now).await)
            .await?;
        Ok(TodoView::from(&todo))
    }
}

async fn change_status<U>(
    uow: &mut U,
    command: ChangeTodoStatusCommand,
    now: Timestamp,
) -> Result<Todo, ChangeTodoStatusError>
where
    U: Repository,
{
    let todo = uow
        .load_undeleted(command.id)
        .await?
        .ok_or(TodoNotFound(command.id))?;
    let TodoByUndeleted::Active(todo) = TodoByUndeleted::from(todo) else {
        return Err(TodoArchived(command.id).into());
    };
    let moved = todo.change_status(command.status, now);
    Ok(uow.save(&moved.into_any()).await?)
}

#[cfg(test)]
#[path = "change_status_tests.rs"]
mod tests;
