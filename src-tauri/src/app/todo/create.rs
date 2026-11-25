use jiff::Timestamp;
use thiserror::Error;
use uuid::Uuid;

use super::TodoFieldError;
use super::view::TodoView;
use crate::app::App;
use crate::app::assignee::AssigneeNotFound;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::error::Violations;
use crate::app::port::Clock;
use crate::app::port::IdGenerator;
use crate::app::port::PortError;
use crate::app::port::UnitOfWorkFactory;
use crate::app::port::assignee;
use crate::app::port::todo;
use crate::domain::assignee::AssigneeId;
use crate::domain::todo::DueAt;
use crate::domain::todo::Todo;
use crate::domain::todo::TodoDescription;
use crate::domain::todo::TodoId;
use crate::domain::todo::TodoTitle;

#[derive(Clone, Debug)]
pub(crate) struct UnvalidatedCreateTodoInput {
    pub(crate) title: String,
    pub(crate) description: Option<String>,
    pub(crate) assignee_id: Option<Uuid>,
    pub(crate) due_at: Option<Timestamp>,
}

#[derive(Debug)]
struct CreateTodoCommand {
    title: TodoTitle,
    description: Option<TodoDescription>,
    assignee_id: Option<AssigneeId>,
    due_at: Option<DueAt>,
}

#[derive(Debug, Error)]
pub(crate) enum CreateTodoError {
    #[error("The input is invalid.")]
    InvalidInput(Violations<TodoFieldError>),
    #[error(transparent)]
    AssigneeNotFound(#[from] AssigneeNotFound),
    #[error(transparent)]
    Port(#[from] PortError),
}

impl AppError for CreateTodoError {
    fn describe(&self) -> ErrorInfo {
        match self {
            Self::InvalidInput(violations) => {
                ErrorInfo::InvalidInput(violations.map_ref(TodoFieldError::describe))
            }
            Self::AssigneeNotFound(error) => super::describe_unknown_assignee(error),
            Self::Port(error) => error.describe(),
        }
    }
}

impl TryFrom<UnvalidatedCreateTodoInput> for CreateTodoCommand {
    type Error = CreateTodoError;

    fn try_from(input: UnvalidatedCreateTodoInput) -> Result<Self, Self::Error> {
        let title = input.title.parse().map_err(TodoFieldError::Title);
        let description = TodoDescription::try_from_optional(input.description.as_deref())
            .map_err(TodoFieldError::Description);

        let violations = match (title, description) {
            (Ok(title), Ok(description)) => {
                return Ok(Self {
                    title,
                    description,
                    assignee_id: input.assignee_id.map(AssigneeId::from),
                    due_at: input.due_at.map(DueAt::from),
                });
            }
            (Err(first), Err(second)) => Violations {
                first,
                rest: vec![second],
            },
            (Err(field), Ok(_)) | (Ok(_), Err(field)) => Violations::one(field),
        };
        Err(CreateTodoError::InvalidInput(violations))
    }
}

impl<S, C, I> App<S, C, I>
where
    S: UnitOfWorkFactory,
    S::UnitOfWork: assignee::Repository + todo::Repository,
    C: Clock,
    I: IdGenerator<TodoId>,
{
    pub(crate) async fn create_todo(
        &self,
        input: UnvalidatedCreateTodoInput,
    ) -> Result<TodoView, CreateTodoError> {
        let command = CreateTodoCommand::try_from(input)?;
        let id = self.id_generator.generate();
        let now = self.clock.now();

        let todo = self
            .transact(async |uow| create(uow, command, id, now).await)
            .await?;
        Ok(TodoView::from(&todo))
    }
}

async fn create<U>(
    uow: &mut U,
    command: CreateTodoCommand,
    id: TodoId,
    now: Timestamp,
) -> Result<Todo, CreateTodoError>
where
    U: assignee::Repository + todo::Repository,
{
    let todo = Todo::new(
        id,
        command.title,
        command.description,
        None,
        command.due_at,
        now,
    );
    let todo = match command.assignee_id {
        None => todo,
        Some(assignee_id) => {
            let assignee = uow
                .load_active(assignee_id)
                .await?
                .ok_or(AssigneeNotFound(assignee_id))?;
            todo.reassign(&assignee)
        }
    };
    Ok(todo::Repository::save(uow, &todo.into_any()).await?)
}

#[cfg(test)]
#[path = "create_tests.rs"]
mod tests;
