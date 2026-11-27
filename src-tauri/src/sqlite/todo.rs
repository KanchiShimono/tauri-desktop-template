use domain::assignee::AssigneeId;
use domain::todo::Active;
use domain::todo::Archived;
use domain::todo::Lifecycle;
use domain::todo::Status;
use domain::todo::StatusState;
use domain::todo::Todo;
use domain::todo::TodoByLifecycle;
use domain::todo::TodoDescription;
use domain::todo::TodoDescriptionError;
use domain::todo::TodoId;
use domain::todo::TodoTitleError;
use domain::todo::Undeleted;
use jiff::Timestamp;
use sqlx::FromRow;
use thiserror::Error;
use uuid::Uuid;
use uuid::fmt::Hyphenated;

use super::pagination::LimitOffset;
use super::store::Store;
use super::timestamp::SqliteTimestamp;
use super::unit_of_work::UnitOfWork;
use crate::app::pagination::ListQuery;
use crate::app::pagination::Page;
use crate::app::port::PortError;
use crate::app::port::todo::Query;
use crate::app::port::todo::Repository;

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

impl TryFrom<TodoRow> for Todo<Active, Status> {
    type Error = TodoRowError;

    fn try_from(row: TodoRow) -> Result<Self, Self::Error> {
        let todo = Todo::try_from(row)?;
        match TodoByLifecycle::from(todo) {
            TodoByLifecycle::Active(todo) => Ok(todo),
            TodoByLifecycle::Archived(_) | TodoByLifecycle::Deleted(_) => {
                Err(TodoRowError::UnexpectedLifecycle)
            }
        }
    }
}

impl TryFrom<TodoRow> for Todo<Archived, Status> {
    type Error = TodoRowError;

    fn try_from(row: TodoRow) -> Result<Self, Self::Error> {
        let todo = Todo::try_from(row)?;
        match TodoByLifecycle::from(todo) {
            TodoByLifecycle::Archived(todo) => Ok(todo),
            TodoByLifecycle::Active(_) | TodoByLifecycle::Deleted(_) => {
                Err(TodoRowError::UnexpectedLifecycle)
            }
        }
    }
}

impl TryFrom<TodoRow> for Todo<Undeleted, Status> {
    type Error = TodoRowError;

    fn try_from(row: TodoRow) -> Result<Self, Self::Error> {
        let todo = Todo::try_from(row)?;
        match TodoByLifecycle::from(todo) {
            TodoByLifecycle::Active(todo) => Ok(todo.into()),
            TodoByLifecycle::Archived(todo) => Ok(todo.into()),
            TodoByLifecycle::Deleted(_) => Err(TodoRowError::UnexpectedLifecycle),
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
    #[error("lifecycle does not match the query")]
    UnexpectedLifecycle,
}

impl From<TodoRowError> for PortError {
    fn from(source: TodoRowError) -> Self {
        Self::Backend(Box::new(source))
    }
}

impl Query for Store {
    async fn find(&self, id: TodoId) -> Result<Option<Todo>, PortError> {
        let row = sqlx::query_as::<_, TodoRow>("SELECT * FROM todos WHERE id = ?")
            .bind(Uuid::from(id).hyphenated())
            .fetch_optional(self.pool())
            .await?;
        Ok(row.map(Todo::try_from).transpose()?)
    }

    async fn list_active(&self) -> Result<Vec<Todo<Active, Status>>, PortError> {
        let rows = sqlx::query_as::<_, TodoRow>(
            "SELECT * FROM todos
             WHERE archived_at IS NULL AND deleted_at IS NULL
             ORDER BY created_at, id",
        )
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(Todo::try_from)
            .collect::<Result<_, _>>()?)
    }

    async fn list_archived(
        &self,
        query: ListQuery,
    ) -> Result<Page<Todo<Archived, Status>>, PortError> {
        let LimitOffset { limit, offset } = query.into();
        let rows = sqlx::query_as::<_, TodoRow>(
            "SELECT * FROM todos
             WHERE archived_at IS NOT NULL AND deleted_at IS NULL
             ORDER BY archived_at DESC, id DESC
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool())
        .await?;
        let todos = rows
            .into_iter()
            .map(Todo::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Page::from_lookahead(query, todos)?)
    }
}

impl Repository for UnitOfWork {
    async fn load_undeleted(
        &mut self,
        id: TodoId,
    ) -> Result<Option<Todo<Undeleted, Status>>, PortError> {
        let row =
            sqlx::query_as::<_, TodoRow>("SELECT * FROM todos WHERE id = ? AND deleted_at IS NULL")
                .bind(Uuid::from(id).hyphenated())
                .fetch_optional(self.connection())
                .await?;
        Ok(row.map(Todo::try_from).transpose()?)
    }

    async fn load_all_for(&mut self, assignee_id: AssigneeId) -> Result<Vec<Todo>, PortError> {
        let rows = sqlx::query_as::<_, TodoRow>(
            "SELECT * FROM todos WHERE assignee_id = ? ORDER BY created_at, id",
        )
        .bind(Uuid::from(assignee_id).hyphenated())
        .fetch_all(self.connection())
        .await?;
        Ok(rows
            .into_iter()
            .map(Todo::try_from)
            .collect::<Result<_, _>>()?)
    }

    async fn save(&mut self, todo: &Todo) -> Result<Todo, PortError> {
        let row = TodoRow::from(todo);
        let stored = sqlx::query_as::<_, TodoRow>(
            "INSERT INTO todos (
                 id, title, description, assignee_id, due_at,
                 created_at, started_at, completed_at, archived_at, deleted_at
             )
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                 title = excluded.title,
                 description = excluded.description,
                 assignee_id = excluded.assignee_id,
                 due_at = excluded.due_at,
                 created_at = excluded.created_at,
                 started_at = excluded.started_at,
                 completed_at = excluded.completed_at,
                 archived_at = excluded.archived_at,
                 deleted_at = excluded.deleted_at
             RETURNING *",
        )
        .bind(row.id)
        .bind(row.title)
        .bind(row.description)
        .bind(row.assignee_id)
        .bind(row.due_at)
        .bind(row.created_at)
        .bind(row.started_at)
        .bind(row.completed_at)
        .bind(row.archived_at)
        .bind(row.deleted_at)
        .fetch_one(self.connection())
        .await?;
        Ok(stored.try_into()?)
    }

    async fn has_undeleted_for(&mut self, assignee_id: AssigneeId) -> Result<bool, PortError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM todos WHERE assignee_id = ? AND deleted_at IS NULL)",
        )
        .bind(Uuid::from(assignee_id).hyphenated())
        .fetch_one(self.connection())
        .await?)
    }
}

#[cfg(test)]
#[path = "todo_tests.rs"]
mod tests;
