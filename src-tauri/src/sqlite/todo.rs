use jiff::Timestamp;
use sqlx::FromRow;
use thiserror::Error;
use uuid::Uuid;
use uuid::fmt::Hyphenated;

use super::store::Store;
use super::timestamp::SqliteTimestamp;
use super::unit_of_work::UnitOfWork;
use crate::app::pagination::ListQuery;
use crate::app::pagination::Page;
use crate::app::port::PortError;
use crate::app::port::todo::Query;
use crate::app::port::todo::Repository;
use crate::domain::assignee::AssigneeId;
use crate::domain::todo::Active;
use crate::domain::todo::Archived;
use crate::domain::todo::Lifecycle;
use crate::domain::todo::Status;
use crate::domain::todo::StatusState;
use crate::domain::todo::Todo;
use crate::domain::todo::TodoDescription;
use crate::domain::todo::TodoDescriptionError;
use crate::domain::todo::TodoId;
use crate::domain::todo::TodoTitleError;
use crate::domain::todo::Undeleted;

#[derive(FromRow)]
struct TodoRow {
    id: Hyphenated,
    title: String,
    description: Option<String>,
    assignee_id: Option<Hyphenated>,
    due_at: Option<SqliteTimestamp>,
    created_at: SqliteTimestamp,
    started_at: Option<SqliteTimestamp>,
    completed_at: Option<SqliteTimestamp>,
    archived_at: Option<SqliteTimestamp>,
    deleted_at: Option<SqliteTimestamp>,
}

impl From<&Todo> for TodoRow {
    fn from(todo: &Todo) -> Self {
        let (started_at, completed_at) = match todo.status() {
            Status::NotStarted(_) => (None, None),
            Status::InProgress(progress) => (Some(progress.started_at()), None),
            Status::Done(done) => (Some(done.started_at()), Some(done.completed_at())),
        };
        let (undeleted, deleted_at) = match todo.lifecycle() {
            Lifecycle::Active(active) => (active.into(), None),
            Lifecycle::Archived(archived) => (archived.into(), None),
            Lifecycle::Deleted(deleted) => (deleted.previous(), Some(deleted.deleted_at())),
        };
        let archived_at = match undeleted {
            Undeleted::Active(_) => None,
            Undeleted::Archived(archived) => Some(archived.archived_at()),
        };

        Self {
            id: Uuid::from(todo.id()).hyphenated(),
            title: todo.title().as_ref().into(),
            description: todo
                .description()
                .map(|description| description.as_ref().into()),
            assignee_id: todo.assignee_id().map(Into::into).map(Uuid::hyphenated),
            due_at: todo.due_at().map(Timestamp::from).map(Into::into),
            created_at: todo.created_at().into(),
            started_at: started_at.map(Timestamp::from).map(Into::into),
            completed_at: completed_at.map(Timestamp::from).map(Into::into),
            archived_at: archived_at.map(Into::into),
            deleted_at: deleted_at.map(Into::into),
        }
    }
}

impl TryFrom<TodoRow> for Todo {
    type Error = TodoRowError;

    fn try_from(row: TodoRow) -> Result<Self, Self::Error> {
        let todo = Todo::new(
            row.id.into_uuid().into(),
            row.title.parse()?,
            TodoDescription::try_from_optional(row.description.as_deref())?,
            row.assignee_id.map(|id| id.into_uuid().into()),
            row.due_at.map(Timestamp::from).map(Into::into),
            row.created_at.into(),
        );
        match (
            row.started_at.map(Timestamp::from).map(Into::into),
            row.completed_at.map(Timestamp::from).map(Into::into),
        ) {
            (None, None) => Ok(row.apply_lifecycle(todo)),
            (Some(started_at), None) => Ok(row.apply_lifecycle(todo.start(started_at))),
            (Some(started_at), Some(completed_at)) => {
                Ok(row.apply_lifecycle(todo.start(started_at).complete(completed_at)))
            }
            (None, Some(_)) => Err(TodoRowError::CompletedWithoutStart),
        }
    }
}

impl TodoRow {
    fn apply_lifecycle<S: StatusState>(&self, todo: Todo<Active, S>) -> Todo {
        match (
            self.archived_at.map(Into::into),
            self.deleted_at.map(Into::into),
        ) {
            (None, None) => todo.into_any(),
            (Some(archived_at), None) => todo.archive(archived_at).into_any(),
            (None, Some(deleted_at)) => todo.delete(deleted_at).into_any(),
            (Some(archived_at), Some(deleted_at)) => {
                todo.archive(archived_at).delete(deleted_at).into_any()
            }
        }
    }
}

#[derive(Debug, Error)]
enum TodoRowError {
    #[error("invalid todo title")]
    InvalidTitle(#[from] TodoTitleError),
    #[error("invalid todo description")]
    InvalidDescription(#[from] TodoDescriptionError),
    #[error("completed_at requires started_at")]
    CompletedWithoutStart,
}

impl Query for Store {
    async fn find(&self, id: TodoId) -> Result<Option<Todo>, PortError> {
        todo!()
    }

    async fn list_active(&self) -> Result<Vec<Todo<Active, Status>>, PortError> {
        todo!()
    }

    async fn list_archived(
        &self,
        _query: ListQuery,
    ) -> Result<Page<Todo<Archived, Status>>, PortError> {
        todo!()
    }
}

impl Repository for UnitOfWork {
    async fn load_undeleted(
        &mut self,
        id: TodoId,
    ) -> Result<Option<Todo<Undeleted, Status>>, PortError> {
        todo!()
    }

    async fn load_all_for(&mut self, assignee_id: AssigneeId) -> Result<Vec<Todo>, PortError> {
        todo!()
    }

    async fn save(&mut self, todo: &Todo) -> Result<Todo, PortError> {
        todo!()
    }

    async fn has_undeleted_for(&mut self, assignee_id: AssigneeId) -> Result<bool, PortError> {
        todo!()
    }
}

#[cfg(test)]
#[path = "todo_tests.rs"]
mod tests;
