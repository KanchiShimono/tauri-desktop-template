use domain::todo::Todo;
use jiff::Timestamp;
use uuid::Uuid;

use crate::app::pagination::UnvalidatedListQuery;
use crate::app::port::todo::Repository;
use crate::app::testing;
use crate::app::todo::view::TodoView;

#[tokio::test]
async fn the_requested_page_of_the_archived_todos_is_listed() {
    let todos = [1, 2, 3, 4].map(|id| {
        Todo::new(
            Uuid::from_u128(id).into(),
            "Review design".parse().expect("valid title"),
            None,
            None,
            None,
            Timestamp::UNIX_EPOCH,
        )
        .archive(Timestamp::UNIX_EPOCH)
        .into_any()
    });
    let app = testing::app();
    for todo in &todos {
        app.transact(async |uow| uow.save(todo).await)
            .await
            .expect("stored todo");
    }

    let page = app
        .list_archived_todos(UnvalidatedListQuery {
            limit: 1,
            offset: 2,
        })
        .await
        .expect("valid query");

    assert_eq!(page.items(), [TodoView::from(&todos[2])]);
    assert_eq!(page.next_offset(), Some(3));
}
