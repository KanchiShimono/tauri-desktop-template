use jiff::Timestamp;
use uuid::Uuid;

use super::ArchiveTodoError;
use super::UnvalidatedArchiveTodoInput;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::port::PortError;
use crate::app::port::todo::Query;
use crate::app::port::todo::Repository;
use crate::app::testing;
use crate::app::testing::TestApp;
use crate::app::todo::view::TodoLifecycleView;
use crate::app::todo::view::TodoView;
use crate::domain::todo::Active;
use crate::domain::todo::NotStarted;
use crate::domain::todo::Todo;

fn at(second: i64) -> Timestamp {
    Timestamp::from_second(second).expect("valid time")
}

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
        assert_eq!(ArchiveTodoError::from(port).describe(), expected);
    }
}

#[tokio::test]
async fn an_active_todo_is_archived_at_the_current_time() {
    let todo = new_todo().into_any();
    let app = app_with(&todo).await;
    app.set_time(at(2));

    let archived = app
        .archive_todo(UnvalidatedArchiveTodoInput {
            id: todo.id().into(),
        })
        .await
        .expect("active todo");

    assert_eq!(
        archived.lifecycle(),
        TodoLifecycleView::Archived { archived_at: at(2) }
    );
    let stored = app.store.find(todo.id()).await.expect("query");
    assert_eq!(stored.as_ref().map(TodoView::from), Some(archived));
}

#[tokio::test]
async fn an_already_archived_todo_is_returned_unchanged() {
    let todo = new_todo().archive(at(1)).into_any();
    let app = app_with(&todo).await;
    app.set_time(at(2));

    let archived = app
        .archive_todo(UnvalidatedArchiveTodoInput {
            id: todo.id().into(),
        })
        .await
        .expect("archived todo");

    assert_eq!(archived, TodoView::from(&todo));
}

#[tokio::test]
async fn a_missing_todo_is_not_found() {
    let app = testing::app();

    let error = app
        .archive_todo(UnvalidatedArchiveTodoInput {
            id: Uuid::from_u128(1),
        })
        .await
        .expect_err("no todo");

    assert!(matches!(
        error.describe(),
        ErrorInfo::NotFound(reason) if reason.code == "todo_not_found"
    ));
}
