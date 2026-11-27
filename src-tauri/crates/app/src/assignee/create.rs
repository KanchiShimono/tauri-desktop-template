use domain::assignee::Assignee;
use domain::assignee::AssigneeId;
use domain::assignee::AssigneeName;
use domain::assignee::AssigneeNameError;
use jiff::Timestamp;
use thiserror::Error;

use super::view::AssigneeView;
use crate::app::App;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::port::Clock;
use crate::app::port::IdGenerator;
use crate::app::port::PortError;
use crate::app::port::UnitOfWorkFactory;
use crate::app::port::assignee::Repository;

#[derive(Clone, Debug)]
pub(crate) struct UnvalidatedCreateAssigneeInput {
    pub(crate) name: String,
}

#[derive(Debug)]
struct CreateAssigneeCommand {
    name: AssigneeName,
}

#[derive(Debug, Error)]
pub(crate) enum CreateAssigneeError {
    #[error("The name is invalid.")]
    InvalidName(#[from] AssigneeNameError),
    #[error(transparent)]
    Port(#[from] PortError),
}

impl AppError for CreateAssigneeError {
    fn describe(&self) -> ErrorInfo {
        match self {
            Self::InvalidName(error) => super::describe_invalid_name(error),
            Self::Port(error) => error.describe(),
        }
    }
}

impl TryFrom<UnvalidatedCreateAssigneeInput> for CreateAssigneeCommand {
    type Error = CreateAssigneeError;

    fn try_from(input: UnvalidatedCreateAssigneeInput) -> Result<Self, Self::Error> {
        Ok(Self {
            name: input.name.parse()?,
        })
    }
}

impl<S, C, I> App<S, C, I>
where
    S: UnitOfWorkFactory,
    S::UnitOfWork: Repository,
    C: Clock,
    I: IdGenerator<AssigneeId>,
{
    pub(crate) async fn create_assignee(
        &self,
        input: UnvalidatedCreateAssigneeInput,
    ) -> Result<AssigneeView, CreateAssigneeError> {
        let command = CreateAssigneeCommand::try_from(input)?;
        let id = self.id_generator.generate();
        let now = self.clock.now();

        let assignee = self
            .transact(async |uow| create(uow, command, id, now).await)
            .await?;
        Ok(AssigneeView::from(&assignee))
    }
}

async fn create<U>(
    uow: &mut U,
    command: CreateAssigneeCommand,
    id: AssigneeId,
    now: Timestamp,
) -> Result<Assignee, CreateAssigneeError>
where
    U: Repository,
{
    let assignee = Assignee::new(id, command.name, now);
    Ok(uow.save(&assignee.into_any()).await?)
}

#[cfg(test)]
#[path = "create_tests.rs"]
mod tests;
