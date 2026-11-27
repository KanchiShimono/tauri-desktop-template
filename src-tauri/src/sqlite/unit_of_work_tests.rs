use std::time::Duration;

use domain::assignee::Assignee;
use jiff::Timestamp;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

use crate::app::port::PortError;
use crate::app::port::UnitOfWork;
use crate::app::port::UnitOfWorkFactory;
use crate::app::port::assignee::Query;
use crate::app::port::assignee::Repository;
use crate::sqlite::store::Store;
use crate::sqlite::test_db;

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

#[sqlx::test]
async fn a_rolled_back_unit_of_work_saves_nothing() {
    let store = test_db::store().await;
    let assignee = Assignee::new(
        Uuid::nil().into(),
        "Ada".parse().expect("valid name"),
        Timestamp::UNIX_EPOCH,
    )
    .into_any();

    let mut uow = store.begin().await.expect("begin");
    uow.save(&assignee).await.expect("save");
    uow.rollback().await.expect("rollback");

    assert_eq!(store.find(assignee.id()).await.expect("query"), None);
}
