use jiff::Timestamp;
use thiserror::Error;
use uuid::Uuid;

use super::AssigneeNotFound;
use super::view::AssigneeView;
use crate::app::App;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::error::PublicReason;
use crate::app::port::Clock;
use crate::app::port::PortError;
use crate::app::port::UnitOfWorkFactory;
use crate::app::port::assignee;
use crate::app::port::todo;
use crate::domain::assignee::Assignee;
use crate::domain::assignee::AssigneeId;

#[derive(Clone, Debug)]
pub(crate) struct UnvalidatedDeleteAssigneeInput {
    pub(crate) id: Uuid,
    pub(crate) assigned_todo_policy: AssignedTodoPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AssignedTodoPolicy {
    Reject,
    Unassign,
}

#[derive(Debug)]
struct DeleteAssigneeCommand {
    id: AssigneeId,
    assigned_todo_policy: AssignedTodoPolicy,
}

#[derive(Debug, Error)]
pub(crate) enum DeleteAssigneeError {
    #[error(transparent)]
    NotFound(#[from] AssigneeNotFound),
    #[error("Assignee {0} still has Todos.")]
    InUse(AssigneeId),
    #[error(transparent)]
    Port(#[from] PortError),
}

impl AppError for DeleteAssigneeError {
    fn describe(&self) -> ErrorInfo {
        match self {
            Self::NotFound(error) => error.describe(),
            Self::InUse(_) => ErrorInfo::Conflict(PublicReason {
                code: "assignee_in_use",
                message: self.to_string(),
            }),
            Self::Port(error) => error.describe(),
        }
    }
}

impl From<UnvalidatedDeleteAssigneeInput> for DeleteAssigneeCommand {
    fn from(input: UnvalidatedDeleteAssigneeInput) -> Self {
        Self {
            id: input.id.into(),
            assigned_todo_policy: input.assigned_todo_policy,
        }
    }
}

impl<S, C, I> App<S, C, I>
where
    S: UnitOfWorkFactory,
    S::UnitOfWork: assignee::Repository + todo::Repository,
    C: Clock,
{
    pub(crate) async fn delete_assignee(
        &self,
        input: UnvalidatedDeleteAssigneeInput,
    ) -> Result<AssigneeView, DeleteAssigneeError> {
        let command = DeleteAssigneeCommand::from(input);
        let now = self.clock.now();

        let assignee = self
            .transact(async |uow| delete(uow, command, now).await)
            .await?;
        Ok(AssigneeView::from(&assignee))
    }
}

async fn delete<U>(
    uow: &mut U,
    command: DeleteAssigneeCommand,
    now: Timestamp,
) -> Result<Assignee, DeleteAssigneeError>
where
    U: assignee::Repository + todo::Repository,
{
    let assignee = uow
        .load_active(command.id)
        .await?
        .ok_or(AssigneeNotFound(command.id))?;
    let blocked_by_todos = match command.assigned_todo_policy {
        AssignedTodoPolicy::Reject => uow.has_undeleted_for(command.id).await?,
        AssignedTodoPolicy::Unassign => false,
    };
    if blocked_by_todos {
        return Err(DeleteAssigneeError::InUse(command.id));
    }
    let todos = uow.load_all_for(command.id).await?;

    let deleted = assignee.delete(now);
    let unassigned = todos
        .into_iter()
        .map(|todo| todo.unassign_for_deleted_assignee(&deleted));

    for todo in unassigned {
        todo::Repository::save(uow, &todo).await?;
    }
    Ok(assignee::Repository::save(uow, &deleted.into_any()).await?)
}

#[cfg(test)]
#[path = "delete_tests.rs"]
mod tests;
