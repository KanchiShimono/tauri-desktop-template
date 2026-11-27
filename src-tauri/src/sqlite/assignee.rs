use domain::assignee::Active;
use domain::assignee::Assignee;
use domain::assignee::AssigneeByLifecycle;
use domain::assignee::AssigneeId;
use domain::assignee::AssigneeNameError;
use domain::assignee::Lifecycle;
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
use crate::app::port::assignee::Query;
use crate::app::port::assignee::Repository;

#[derive(FromRow)]
struct AssigneeRow {
    id: Hyphenated,
    name: String,
    created_at: SqliteTimestamp,
    deleted_at: Option<SqliteTimestamp>,
}

impl From<&Assignee> for AssigneeRow {
    fn from(assignee: &Assignee) -> Self {
        Self {
            id: Uuid::from(assignee.id()).hyphenated(),
            name: assignee.name().as_ref().into(),
            created_at: assignee.created_at().into(),
            deleted_at: match assignee.lifecycle() {
                Lifecycle::Active(_) => None,
                Lifecycle::Deleted(deleted) => Some(deleted.deleted_at().into()),
            },
        }
    }
}

impl TryFrom<AssigneeRow> for Assignee {
    type Error = AssigneeRowError;

    fn try_from(row: AssigneeRow) -> Result<Self, Self::Error> {
        let assignee = Assignee::new(
            row.id.into_uuid().into(),
            row.name.parse()?,
            row.created_at.into(),
        );
        Ok(match row.deleted_at {
            None => assignee.into_any(),
            Some(deleted_at) => assignee.delete(deleted_at.into()).into_any(),
        })
    }
}

impl TryFrom<AssigneeRow> for Assignee<Active> {
    type Error = AssigneeRowError;

    fn try_from(row: AssigneeRow) -> Result<Self, Self::Error> {
        let assignee = Assignee::try_from(row)?;
        match AssigneeByLifecycle::from(assignee) {
            AssigneeByLifecycle::Active(assignee) => Ok(assignee),
            AssigneeByLifecycle::Deleted(_) => Err(AssigneeRowError::UnexpectedLifecycle),
        }
    }
}

#[derive(Debug, Error)]
enum AssigneeRowError {
    #[error("invalid assignee name")]
    InvalidName(#[from] AssigneeNameError),
    #[error("lifecycle does not match the query")]
    UnexpectedLifecycle,
}

impl From<AssigneeRowError> for PortError {
    fn from(source: AssigneeRowError) -> Self {
        Self::Backend(Box::new(source))
    }
}

impl Query for Store {
    async fn find(&self, id: AssigneeId) -> Result<Option<Assignee>, PortError> {
        let row = sqlx::query_as::<_, AssigneeRow>("SELECT * FROM assignees WHERE id = ?")
            .bind(Uuid::from(id).hyphenated())
            .fetch_optional(self.pool())
            .await?;
        Ok(row.map(Assignee::try_from).transpose()?)
    }

    async fn list_active(&self, query: ListQuery) -> Result<Page<Assignee<Active>>, PortError> {
        let LimitOffset { limit, offset } = query.into();
        let rows = sqlx::query_as::<_, AssigneeRow>(
            "SELECT * FROM assignees
             WHERE deleted_at IS NULL
             ORDER BY created_at, id
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool())
        .await?;
        let assignees = rows
            .into_iter()
            .map(Assignee::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Page::from_lookahead(query, assignees)?)
    }
}

impl Repository for UnitOfWork {
    async fn load_active(&mut self, id: AssigneeId) -> Result<Option<Assignee<Active>>, PortError> {
        let row = sqlx::query_as::<_, AssigneeRow>(
            "SELECT * FROM assignees WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(Uuid::from(id).hyphenated())
        .fetch_optional(self.connection())
        .await?;
        Ok(row.map(Assignee::try_from).transpose()?)
    }

    async fn save(&mut self, assignee: &Assignee) -> Result<Assignee, PortError> {
        let row = AssigneeRow::from(assignee);
        let stored = sqlx::query_as::<_, AssigneeRow>(
            "INSERT INTO assignees (id, name, created_at, deleted_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                 name = excluded.name,
                 created_at = excluded.created_at,
                 deleted_at = excluded.deleted_at
             RETURNING *",
        )
        .bind(row.id)
        .bind(row.name)
        .bind(row.created_at)
        .bind(row.deleted_at)
        .fetch_one(self.connection())
        .await?;
        Ok(stored.try_into()?)
    }
}

#[cfg(test)]
#[path = "assignee_tests.rs"]
mod tests;
