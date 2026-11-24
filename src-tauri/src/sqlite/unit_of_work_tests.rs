use std::time::Duration;

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqlitePoolOptions;

use crate::app::port::PortError;
use crate::app::port::UnitOfWork;
use crate::app::port::UnitOfWorkFactory;
use crate::sqlite::store::Store;

#[sqlx::test]
async fn a_unit_of_work_holds_the_write_lock_from_its_start(
    pool_options: SqlitePoolOptions,
    connect_options: SqliteConnectOptions,
) {
    let pool = pool_options
        .connect_with(connect_options.busy_timeout(Duration::ZERO))
        .await
        .expect("open pool");
    let store = Store::new(pool);

    let first = store.begin().await.expect("first unit of work");
    let second = store.begin().await;
    assert!(matches!(second, Err(PortError::Busy(_))), "{second:?}");

    first.rollback().await.expect("rollback");
}
