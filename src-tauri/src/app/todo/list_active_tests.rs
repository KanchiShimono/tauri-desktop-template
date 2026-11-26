use jiff::Timestamp;
use uuid::Uuid;

use crate::app::port::todo::Repository;
use crate::app::testing;
use crate::app::todo::view::TodoView;
use crate::domain::todo::Todo;

#[tokio::test]
async fn the_active_todos_are_listed() {
    let todo = Todo::new(
        Uuid::from_u128(1).into(),
        "Review design".parse().expect("valid title"),
        None,
        None,
        None,
        Timestamp::UNIX_EPOCH,
    )
    .into_any();
    let app = testing::app();
    app.transact(async |uow| uow.save(&todo).await)
        .await
        .expect("stored todo");

    let page = app.list_active_todos().await.expect("query");

    assert_eq!(page.items(), [TodoView::from(&todo)]);
}
