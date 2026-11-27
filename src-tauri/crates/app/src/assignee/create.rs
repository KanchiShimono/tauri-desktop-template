use domain::assignee::Assignee;
use domain::assignee::AssigneeId;
use domain::assignee::AssigneeName;
use domain::assignee::AssigneeNameError;
use jiff::Timestamp;
use thiserror::Error;

use crate::app::App;
use crate::assignee;
use crate::assignee::view::AssigneeView;
use crate::error::AppError;
use crate::error::ErrorInfo;
use crate::port::Clock;
use crate::port::IdGenerator;
use crate::port::PortError;
use crate::port::UnitOfWorkFactory;
use crate::port::assignee::Repository;

#[derive(Clone, Debug)]
pub struct UnvalidatedCreateAssigneeInput {
    pub name: String,
}

#[derive(Debug)]
struct CreateAssigneeCommand {
    name: AssigneeName,
}

#[derive(Debug, Error)]
pub enum CreateAssigneeError {
    #[error("The name is invalid.")]
    InvalidName(#[from] AssigneeNameError),
    #[error(transparent)]
    Port(#[from] PortError),
}

impl AppError for CreateAssigneeError {
    fn describe(&self) -> ErrorInfo {
        match self {
            Self::InvalidName(error) => assignee::describe_invalid_name(error),
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
    pub async fn create_assignee(
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
