use domain::todo::Todo;
use jiff::Timestamp;
use uuid::Uuid;

use super::GetTodoError;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::port::PortError;
use crate::app::port::todo::Repository;
use crate::app::testing;
use crate::app::todo::view::TodoView;

#[test]
fn a_storage_failure_is_described_by_the_port() {
    for (port, expected) in [
        (PortError::Busy("locked".into()), ErrorInfo::Unavailable),
        (PortError::Backend("corrupt".into()), ErrorInfo::Internal),
    ] {
        assert_eq!(GetTodoError::from(port).describe(), expected);
    }
}

#[tokio::test]
async fn a_deleted_todo_is_still_returned() {
    let todo = Todo::new(
        Uuid::from_u128(1).into(),
        "Review design".parse().expect("valid title"),
        None,
        None,
        None,
        Timestamp::UNIX_EPOCH,
    )
    .delete(Timestamp::UNIX_EPOCH)
    .into_any();
    let app = testing::app();
    app.transact(async |uow| uow.save(&todo).await)
        .await
        .expect("stored todo");

    let found = app.get_todo(todo.id().into()).await.expect("stored todo");

    assert_eq!(found, TodoView::from(&todo));
}

#[tokio::test]
async fn a_missing_todo_is_not_found() {
    let app = testing::app();

    let error = app.get_todo(Uuid::from_u128(1)).await.expect_err("no todo");

    assert!(matches!(
        error.describe(),
        ErrorInfo::NotFound(reason) if reason.code == "todo_not_found"
    ));
}
