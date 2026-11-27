use jiff::Timestamp;
use sqlx::Connection;
use sqlx::Error;
use sqlx::SqliteConnection;
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

use super::store::MIGRATOR;
use super::store::Store;
use super::timestamp::SqliteTimestamp;

pub(super) async fn open() -> SqliteConnection {
    SqliteConnection::connect("sqlite::memory:")
        .await
        .expect("open in-memory SQLite database")
}

pub(super) async fn migrated() -> SqliteConnection {
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

#[sqlx::test]
async fn migration_enforces_timestamp_storage_and_is_reversible() {
    let mut db = migrated().await;
    let epoch = SqliteTimestamp::from(Timestamp::UNIX_EPOCH);
    let assignee = Uuid::nil().hyphenated();
    sqlx::query("INSERT INTO assignees (id, name, created_at) VALUES (?, 'Ada', ?)")
        .bind(assignee)
        .bind(epoch)
        .execute(&mut db)
        .await
        .expect("insert assignee");
    sqlx::query("INSERT INTO todos (id, title, assignee_id, created_at) VALUES (?, 'Task', ?, ?)")
        .bind(Uuid::max().hyphenated())
        .bind(assignee)
        .bind(epoch)
        .execute(&mut db)
        .await
        .expect("insert assigned todo");

    for (update, nullable) in [
        ("UPDATE assignees SET created_at = ?", false),
        ("UPDATE assignees SET deleted_at = ?", true),
        ("UPDATE todos SET created_at = ?", false),
        ("UPDATE todos SET due_at = ?", true),
        ("UPDATE todos SET started_at = ?, completed_at = NULL", true),
        (
            "UPDATE todos SET completed_at = ?, started_at = created_at",
            true,
        ),
        ("UPDATE todos SET archived_at = ?", true),
        ("UPDATE todos SET deleted_at = ?", true),
    ] {
        let null = sqlx::query(update)
            .bind(None::<SqliteTimestamp>)
            .execute(&mut db)
            .await;
        assert_eq!(null.is_ok(), nullable, "{update} with NULL: {null:?}");
        sqlx::query(update)
            .bind(epoch)
            .execute(&mut db)
            .await
            .expect(update);

        for (case, query) in [
            ("15-byte BLOB", sqlx::query(update).bind([0_u8; 15])),
            ("17-byte BLOB", sqlx::query(update).bind([0_u8; 17])),
            ("16-byte TEXT", sqlx::query(update).bind("0123456789abcdef")),
        ] {
            let result = query.execute(&mut db).await;
            assert!(
                matches!(result, Err(Error::Database(_))),
                "{update} with {case}: {result:?}"
            );
        }
    }

    MIGRATOR
        .undo(&mut db, 0)
        .await
        .expect("revert populated schema");
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sqlite_schema WHERE name IN ('assignees', 'todos')",
    )
    .fetch_one(&mut db)
    .await
    .expect("inspect reverted schema");
    assert_eq!(remaining, 0);
    MIGRATOR.run(&mut db).await.expect("reapply migrations");
}
