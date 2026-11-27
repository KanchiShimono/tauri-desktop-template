use domain::assignee::Assignee;
use domain::assignee::AssigneeId;
use domain::assignee::AssigneeName;
use domain::assignee::AssigneeNameError;
use thiserror::Error;
use uuid::Uuid;

use super::AssigneeNotFound;
use super::view::AssigneeView;
use crate::app::App;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::port::PortError;
use crate::app::port::UnitOfWorkFactory;
use crate::app::port::assignee::Repository;

#[derive(Clone, Debug)]
pub(crate) struct UnvalidatedRenameAssigneeInput {
    pub(crate) id: Uuid,
    pub(crate) name: String,
}

#[derive(Debug)]
struct RenameAssigneeCommand {
    id: AssigneeId,
    name: AssigneeName,
}

#[derive(Debug, Error)]
pub(crate) enum RenameAssigneeError {
    #[error("The name is invalid.")]
    InvalidName(#[from] AssigneeNameError),
    #[error(transparent)]
    NotFound(#[from] AssigneeNotFound),
    #[error(transparent)]
    Port(#[from] PortError),
}

impl AppError for RenameAssigneeError {
    fn describe(&self) -> ErrorInfo {
        match self {
            Self::InvalidName(error) => super::describe_invalid_name(error),
            Self::NotFound(error) => error.describe(),
            Self::Port(error) => error.describe(),
        }
    }
}

impl TryFrom<UnvalidatedRenameAssigneeInput> for RenameAssigneeCommand {
    type Error = RenameAssigneeError;

    fn try_from(input: UnvalidatedRenameAssigneeInput) -> Result<Self, Self::Error> {
        Ok(Self {
            id: input.id.into(),
            name: input.name.parse()?,
        })
    }
}

impl<S, C, I> App<S, C, I>
where
    S: UnitOfWorkFactory,
    S::UnitOfWork: Repository,
{
    pub(crate) async fn rename_assignee(
        &self,
        input: UnvalidatedRenameAssigneeInput,
    ) -> Result<AssigneeView, RenameAssigneeError> {
        let command = RenameAssigneeCommand::try_from(input)?;

        let assignee = self
            .transact(async |uow| rename(uow, command).await)
            .await?;
        Ok(AssigneeView::from(&assignee))
    }
}

async fn rename<U>(
    uow: &mut U,
    command: RenameAssigneeCommand,
) -> Result<Assignee, RenameAssigneeError>
where
    U: Repository,
{
    let assignee = uow
        .load_active(command.id)
        .await?
        .ok_or(AssigneeNotFound(command.id))?;
    let renamed = assignee.rename(command.name);
    Ok(uow.save(&renamed.into_any()).await?)
}

#[cfg(test)]
#[path = "rename_tests.rs"]
mod tests;
