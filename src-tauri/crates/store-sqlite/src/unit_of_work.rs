use app::port;
use app::port::PortError;
use app::port::UnitOfWorkFactory;
use sqlx::Sqlite;
use sqlx::SqliteConnection;
use sqlx::Transaction;

use crate::error;
use crate::store::Store;

#[derive(Debug)]
pub struct UnitOfWork(Transaction<'static, Sqlite>);

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
        self.0.commit().await.map_err(error::into_port_error)
    }

    async fn rollback(self) -> Result<(), PortError> {
        self.0.rollback().await.map_err(error::into_port_error)
    }
}

impl UnitOfWorkFactory for Store {
    type UnitOfWork = UnitOfWork;

    async fn begin(&self) -> Result<Self::UnitOfWork, PortError> {
        self.pool()
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map(UnitOfWork::new)
            .map_err(error::into_port_error)
    }
}

#[cfg(test)]
#[path = "unit_of_work_tests.rs"]
mod tests;
