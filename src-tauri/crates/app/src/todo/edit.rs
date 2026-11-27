use domain::assignee::AssigneeId;
use domain::todo::Active;
use domain::todo::CompletedAt;
use domain::todo::Done;
use domain::todo::DueAt;
use domain::todo::InProgress;
use domain::todo::StartedAt;
use domain::todo::Status;
use domain::todo::Todo;
use domain::todo::TodoByStatus;
use domain::todo::TodoByUndeleted;
use domain::todo::TodoDescription;
use domain::todo::TodoId;
use domain::todo::TodoTitle;
use jiff::Timestamp;
use thiserror::Error;
use uuid::Uuid;

use super::TodoArchived;
use super::TodoFieldError;
use super::TodoNotFound;
use super::view::TodoView;
use crate::app::App;
use crate::app::assignee::AssigneeNotFound;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::error::FieldViolation;
use crate::app::error::Violations;
use crate::app::port::PortError;
use crate::app::port::UnitOfWorkFactory;
use crate::app::port::assignee;
use crate::app::port::todo;
use crate::app::presence::Presence;

#[derive(Clone, Debug)]
pub(crate) struct UnvalidatedEditTodoInput {
    pub(crate) id: Uuid,
    pub(crate) title: Presence<String>,
    pub(crate) description: Presence<Option<String>>,
    pub(crate) assignee_id: Presence<Option<Uuid>>,
    pub(crate) due_at: Presence<Option<Timestamp>>,
    pub(crate) started_at: Presence<Timestamp>,
    pub(crate) completed_at: Presence<Timestamp>,
}

#[derive(Debug)]
struct EditTodoCommand {
    id: TodoId,
    title: Presence<TodoTitle>,
    description: Presence<Option<TodoDescription>>,
    assignee_id: Presence<Option<AssigneeId>>,
    due_at: Presence<Option<DueAt>>,
    started_at: Presence<StartedAt>,
    completed_at: Presence<CompletedAt>,
}

#[derive(Debug, Error)]
pub(crate) enum EditTodoError {
    #[error("The input is invalid.")]
    InvalidInput(Violations<TodoFieldError>),
    #[error(transparent)]
    NotFound(#[from] TodoNotFound),
    #[error(transparent)]
    Archived(#[from] TodoArchived),
    #[error(transparent)]
    AssigneeNotFound(#[from] AssigneeNotFound),
    #[error("The start time can be changed only after the Todo has been started.")]
    NotStarted,
    #[error("The completion time can be changed only after the Todo has been completed.")]
    NotCompleted,
    #[error(transparent)]
    Port(#[from] PortError),
}

impl AppError for EditTodoError {
    fn describe(&self) -> ErrorInfo {
        match self {
            Self::InvalidInput(violations) => {
                ErrorInfo::InvalidInput(violations.map_ref(TodoFieldError::describe))
            }
            Self::NotFound(error) => error.describe(),
            Self::Archived(error) => error.describe(),
            Self::AssigneeNotFound(error) => super::describe_unknown_assignee(error),
            Self::NotStarted => ErrorInfo::InvalidInput(Violations::one(FieldViolation::new(
                "started_at",
                "not_started",
                self.to_string(),
            ))),
            Self::NotCompleted => ErrorInfo::InvalidInput(Violations::one(FieldViolation::new(
                "completed_at",
                "not_completed",
                self.to_string(),
            ))),
            Self::Port(error) => error.describe(),
        }
    }
}

impl TryFrom<UnvalidatedEditTodoInput> for EditTodoCommand {
    type Error = EditTodoError;

    fn try_from(input: UnvalidatedEditTodoInput) -> Result<Self, Self::Error> {
        let title = input
            .title
            .map(|title| title.parse().map_err(TodoFieldError::Title))
            .transpose();
        let description = input
            .description
            .map(|description| {
                TodoDescription::try_from_optional(description.as_deref())
                    .map_err(TodoFieldError::Description)
            })
            .transpose();

        let violations = match (title, description) {
            (Ok(title), Ok(description)) => {
                return Ok(Self {
                    id: input.id.into(),
                    title,
                    description,
                    assignee_id: input.assignee_id.map(|id| id.map(AssigneeId::from)),
                    due_at: input.due_at.map(|due_at| due_at.map(DueAt::from)),
                    started_at: input.started_at.map(StartedAt::from),
                    completed_at: input.completed_at.map(CompletedAt::from),
                });
            }
            (Err(first), Err(second)) => Violations {
                first,
                rest: vec![second],
            },
            (Err(field), Ok(_)) | (Ok(_), Err(field)) => Violations::one(field),
        };
        Err(EditTodoError::InvalidInput(violations))
    }
}

impl<S, C, I> App<S, C, I>
where
    S: UnitOfWorkFactory,
    S::UnitOfWork: assignee::Repository + todo::Repository,
{
    pub(crate) async fn edit_todo(
        &self,
        input: UnvalidatedEditTodoInput,
    ) -> Result<TodoView, EditTodoError> {
        let command = EditTodoCommand::try_from(input)?;

        let todo = self.transact(async |uow| edit(uow, command).await).await?;
        Ok(TodoView::from(&todo))
    }
}

async fn edit<U>(uow: &mut U, command: EditTodoCommand) -> Result<Todo, EditTodoError>
where
    U: assignee::Repository + todo::Repository,
{
    let todo = uow
        .load_undeleted(command.id)
        .await?
        .ok_or(TodoNotFound(command.id))?;
    let TodoByUndeleted::Active(todo) = TodoByUndeleted::from(todo) else {
        return Err(TodoArchived(command.id).into());
    };

    let assignee_change = match command.assignee_id {
        Presence::Absent => Presence::Absent,
        Presence::Present(None) => Presence::Present(None),
        Presence::Present(Some(id)) => {
            let assignee = uow.load_active(id).await?.ok_or(AssigneeNotFound(id))?;
            Presence::Present(Some(assignee))
        }
    };

    let todo =
        Option::from(assignee_change)
            .into_iter()
            .fold(todo, |todo, assignee| match assignee {
                None => todo.unassign(),
                Some(assignee) => todo.reassign(&assignee),
            });
    let todo = Option::from(command.title)
        .into_iter()
        .fold(todo, Todo::rename);
    let todo = Option::from(command.description)
        .into_iter()
        .fold(todo, Todo::change_description);
    let todo = Option::from(command.due_at)
        .into_iter()
        .fold(todo, Todo::change_due_at);
    let todo = change_recorded_times(todo, command.started_at, command.completed_at)?;

    Ok(todo::Repository::save(uow, &todo).await?)
}

fn change_recorded_times(
    todo: Todo<Active, Status>,
    started_at: Presence<StartedAt>,
    completed_at: Presence<CompletedAt>,
) -> Result<Todo, EditTodoError> {
    match TodoByStatus::from(todo) {
        TodoByStatus::NotStarted(todo) => {
            if started_at.is_present() {
                return Err(EditTodoError::NotStarted);
            }
            if completed_at.is_present() {
                return Err(EditTodoError::NotCompleted);
            }
            Ok(todo.into_any())
        }
        TodoByStatus::InProgress(todo) => {
            if completed_at.is_present() {
                return Err(EditTodoError::NotCompleted);
            }
            let todo = Option::from(started_at)
                .into_iter()
                .fold(todo, Todo::<Active, InProgress>::change_started_at);
            Ok(todo.into_any())
        }
        TodoByStatus::Done(todo) => {
            let todo = Option::from(started_at)
                .into_iter()
                .fold(todo, Todo::<Active, Done>::change_started_at);
            let todo = Option::from(completed_at)
                .into_iter()
                .fold(todo, Todo::change_completed_at);
            Ok(todo.into_any())
        }
    }
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;
