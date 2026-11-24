use sqlx::Sqlite;
use sqlx::SqliteConnection;
use sqlx::Transaction;

use crate::app::port;
use crate::app::port::PortError;
use crate::app::port::UnitOfWorkFactory;
use crate::sqlite::store::Store;

#[derive(Debug)]
pub(crate) struct UnitOfWork(Transaction<'static, Sqlite>);

impl UnitOfWork {
    const fn new(transaction: Transaction<'static, Sqlite>) -> Self {
        Self(transaction)
    }

    pub(crate) fn connection(&mut self) -> &mut SqliteConnection {
        self.0.as_mut()
    }
}

impl port::UnitOfWork for UnitOfWork {
    async fn commit(self) -> Result<(), PortError> {
        self.0.commit().await.map_err(Into::into)
    }

    async fn rollback(self) -> Result<(), PortError> {
        self.0.rollback().await.map_err(Into::into)
    }
}

impl UnitOfWorkFactory for Store {
    type UnitOfWork = UnitOfWork;

    async fn begin(&self) -> Result<Self::UnitOfWork, PortError> {
        self.pool()
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map(UnitOfWork::new)
            .map_err(Into::into)
    }
}

#[cfg(test)]
#[path = "unit_of_work_tests.rs"]
mod tests;
