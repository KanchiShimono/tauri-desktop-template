use sqlx::Connection;
use sqlx::SqliteConnection;
use sqlx::sqlite::SqlitePoolOptions;

use crate::store::MIGRATOR;
use crate::store::Store;

pub(crate) async fn open() -> SqliteConnection {
    SqliteConnection::connect("sqlite::memory:")
        .await
        .expect("open in-memory SQLite database")
}

pub(crate) async fn migrated() -> SqliteConnection {
    let mut db = open().await;
    MIGRATOR.run(&mut db).await.expect("apply migrations");
    db
}

pub(crate) async fn store() -> Store {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("open in-memory SQLite pool");
    MIGRATOR.run(&pool).await.expect("apply migrations");
    Store::new(pool)
}

#[cfg(test)]
#[path = "test_db_tests.rs"]
mod tests;
