use domain::todo::Todo;
use jiff::Timestamp;
use uuid::Uuid;

use super::testing;
use crate::app::port::PortError;
use crate::app::port::todo::Query;
use crate::app::port::todo::Repository;

fn todo(id: u128) -> Todo {
    Todo::new(
        Uuid::from_u128(id).into(),
        "Task".parse().expect("valid title"),
        None,
        None,
        None,
        Timestamp::UNIX_EPOCH,
    )
    .into_any()
}

#[tokio::test]
async fn work_is_committed_when_it_succeeds_and_rolled_back_when_it_fails() {
    let app = testing::app();
    let (kept, discarded) = (todo(1), todo(2));

    app.transact(async |uow| uow.save(&kept).await)
        .await
        .expect("committed");
    let failed = app
        .transact(async |uow| {
            uow.save(&discarded).await?;
            Err::<(), _>(PortError::Backend("the work failed".into()))
        })
        .await;

    assert!(failed.is_err());
    assert_eq!(app.store.find(kept.id()).await.expect("query"), Some(kept));
    assert_eq!(app.store.find(discarded.id()).await.expect("query"), None);
}
