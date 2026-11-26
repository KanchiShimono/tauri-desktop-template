use jiff::Timestamp;
use uuid::Uuid;

use super::DeleteTodoError;
use super::UnvalidatedDeleteTodoInput;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::port::PortError;
use crate::app::port::todo::Query;
use crate::app::port::todo::Repository;
use crate::app::testing;
use crate::app::todo::view::TodoLifecycleView;
use crate::app::todo::view::TodoView;
use crate::domain::todo::Todo;

#[test]
fn a_storage_failure_is_described_by_the_port() {
    for (port, expected) in [
        (PortError::Busy("locked".into()), ErrorInfo::Unavailable),
        (PortError::Backend("corrupt".into()), ErrorInfo::Internal),
    ] {
        assert_eq!(DeleteTodoError::from(port).describe(), expected);
    }
}

#[tokio::test]
async fn an_active_or_archived_todo_is_deleted_at_the_current_time() {
    let now = Timestamp::from_second(1_000).expect("valid time");
    let active = Todo::new(
        Uuid::from_u128(1).into(),
        "Review design".parse().expect("valid title"),
        None,
        None,
        None,
        Timestamp::UNIX_EPOCH,
    );
    let archived = active.clone().archive(Timestamp::UNIX_EPOCH);
    for todo in [active.into_any(), archived.into_any()] {
        let app = testing::app();
        app.transact(async |uow| uow.save(&todo).await)
            .await
            .expect("stored todo");
        app.set_time(now);

        let deleted = app
            .delete_todo(UnvalidatedDeleteTodoInput {
                id: todo.id().into(),
            })
            .await
            .expect("undeleted todo");

        assert!(matches!(
            deleted.lifecycle(),
            TodoLifecycleView::Deleted { deleted_at, .. } if deleted_at == now
        ));
        let stored = app.store.find(todo.id()).await.expect("query");
        assert_eq!(stored.as_ref().map(TodoView::from), Some(deleted));
    }
}

#[tokio::test]
async fn a_missing_todo_is_not_found() {
    let app = testing::app();

    let error = app
        .delete_todo(UnvalidatedDeleteTodoInput {
            id: Uuid::from_u128(1),
        })
        .await
        .expect_err("no todo");

    assert!(matches!(
        error.describe(),
        ErrorInfo::NotFound(reason) if reason.code == "todo_not_found"
    ));
}
