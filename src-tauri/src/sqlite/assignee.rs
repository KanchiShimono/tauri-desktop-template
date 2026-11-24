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
use crate::app::port::assignee::Query;
use crate::app::port::assignee::Repository;
use crate::domain::assignee::Active;
use crate::domain::assignee::Assignee;
use crate::domain::assignee::AssigneeId;
use crate::domain::assignee::AssigneeNameError;
use crate::domain::assignee::Lifecycle;

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

#[derive(Debug, Error)]
enum AssigneeRowError {
    #[error("invalid assignee name")]
    InvalidName(#[from] AssigneeNameError),
}

impl Query for Store {
    async fn find(&self, id: AssigneeId) -> Result<Option<Assignee>, PortError> {
        todo!()
    }

    async fn list_active(&self, _query: ListQuery) -> Result<Page<Assignee<Active>>, PortError> {
        todo!()
    }
}

impl Repository for UnitOfWork {
    async fn load_active(&mut self, id: AssigneeId) -> Result<Option<Assignee<Active>>, PortError> {
        todo!()
    }

    async fn save(&mut self, assignee: &Assignee) -> Result<Assignee, PortError> {
        todo!()
    }
}

#[cfg(test)]
#[path = "assignee_tests.rs"]
mod tests;
