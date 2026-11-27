use std::io;
use std::path::Path;

use sqlx::SqlitePool;
use sqlx::migrate::MigrateError;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqliteJournalMode;
use sqlx::sqlite::SqlitePoolOptions;
use thiserror::Error;
use tokio::fs;

pub(super) static MIGRATOR: Migrator = sqlx::migrate!();

#[derive(Clone, Debug)]
pub(crate) struct Store(SqlitePool);

#[derive(Debug, Error)]
pub(crate) enum OpenError {
    #[error("failed to create the database directory")]
    CreateDir(#[source] io::Error),
    #[error("failed to open the database")]
    Connect(#[source] sqlx::Error),
    #[error("failed to migrate the database")]
    Migrate(#[source] MigrateError),
}

impl Store {
    pub const fn new(pool: SqlitePool) -> Self {
        Self(pool)
    }

    pub(crate) async fn open(path: &Path) -> Result<Self, OpenError> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)
                .await
                .map_err(OpenError::CreateDir)?;
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .map_err(OpenError::Connect)?;
        MIGRATOR.run(&pool).await.map_err(OpenError::Migrate)?;
        Ok(Self::new(pool))
    }

    pub(crate) const fn pool(&self) -> &SqlitePool {
        &self.0
    }
}
