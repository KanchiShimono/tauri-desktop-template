use domain::todo::Active;
use domain::todo::NotStarted;
use domain::todo::Todo;
use jiff::Timestamp;
use uuid::Uuid;

use crate::error::AppError;
use crate::error::ErrorInfo;
use crate::port::PortError;
use crate::port::todo::Query;
use crate::port::todo::Repository;
use crate::testing;
use crate::testing::TestApp;
use crate::todo::unarchive::UnarchiveTodoError;
use crate::todo::unarchive::UnvalidatedUnarchiveTodoInput;
use crate::todo::view::TodoLifecycleView;
use crate::todo::view::TodoView;

fn new_todo() -> Todo<Active, NotStarted> {
    Todo::new(
        Uuid::from_u128(1).into(),
        "Review design".parse().expect("valid title"),
        None,
        None,
        None,
        Timestamp::UNIX_EPOCH,
    )
}

async fn app_with(todo: &Todo) -> TestApp {
    let app = testing::app();
    app.transact(async |uow| uow.save(todo).await)
        .await
        .expect("stored todo");
    app
}

#[test]
fn a_storage_failure_is_described_by_the_port() {
    for (port, expected) in [
        (PortError::Busy("locked".into()), ErrorInfo::Unavailable),
        (PortError::Backend("corrupt".into()), ErrorInfo::Internal),
    ] {
        assert_eq!(UnarchiveTodoError::from(port).describe(), expected);
    }
}

#[tokio::test]
async fn an_archived_todo_becomes_active_again() {
    let todo = new_todo().archive(Timestamp::UNIX_EPOCH).into_any();
    let app = app_with(&todo).await;

    let unarchived = app
        .unarchive_todo(UnvalidatedUnarchiveTodoInput {
            id: todo.id().into(),
        })
        .await
        .expect("archived todo");

    assert_eq!(unarchived.lifecycle(), TodoLifecycleView::Active);
    let stored = app.store.find(todo.id()).await.expect("query");
    assert_eq!(stored.as_ref().map(TodoView::from), Some(unarchived));
}

#[tokio::test]
async fn an_already_active_todo_is_returned_unchanged() {
    let todo = new_todo().into_any();
    let app = app_with(&todo).await;

    let unarchived = app
        .unarchive_todo(UnvalidatedUnarchiveTodoInput {
            id: todo.id().into(),
        })
        .await
        .expect("active todo");

    assert_eq!(unarchived, TodoView::from(&todo));
}

#[tokio::test]
async fn a_missing_todo_is_not_found() {
    let app = testing::app();

    let error = app
        .unarchive_todo(UnvalidatedUnarchiveTodoInput {
            id: Uuid::from_u128(1),
        })
        .await
        .expect_err("no todo");

    assert!(matches!(
        error.describe(),
        ErrorInfo::NotFound(reason) if reason.code == "todo_not_found"
    ));
}
