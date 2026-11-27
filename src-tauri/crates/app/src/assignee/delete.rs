use domain::assignee::Assignee;
use domain::assignee::AssigneeId;
use jiff::Timestamp;
use thiserror::Error;
use uuid::Uuid;

use crate::app::App;
use crate::assignee::AssigneeNotFound;
use crate::assignee::view::AssigneeView;
use crate::error::AppError;
use crate::error::ErrorInfo;
use crate::error::PublicReason;
use crate::port::Clock;
use crate::port::PortError;
use crate::port::UnitOfWorkFactory;
use crate::port::assignee;
use crate::port::todo;

#[derive(Clone, Debug)]
pub struct UnvalidatedDeleteAssigneeInput {
    pub id: Uuid,
    pub assigned_todo_policy: AssignedTodoPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssignedTodoPolicy {
    Reject,
    Unassign,
}

#[derive(Debug)]
struct DeleteAssigneeCommand {
    id: AssigneeId,
    assigned_todo_policy: AssignedTodoPolicy,
}

#[derive(Debug, Error)]
pub enum DeleteAssigneeError {
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
    pub async fn delete_assignee(
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
