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
use crate::todo::change_status::ChangeTodoStatusError;
use crate::todo::change_status::TodoStatusInput;
use crate::todo::change_status::UnvalidatedChangeTodoStatusInput;
use crate::todo::view::TodoStatusView;
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
        assert_eq!(ChangeTodoStatusError::from(port).describe(), expected);
    }
}

#[tokio::test]
async fn a_moved_todo_is_stored_in_the_target_column_with_the_current_time() {
    let now = Timestamp::from_second(1_000).expect("valid time");
    let in_progress = TodoStatusView::InProgress { started_at: now };
    let done = TodoStatusView::Done {
        started_at: now,
        completed_at: now,
    };
    for (status, expected) in [
        (TodoStatusInput::NotStarted, TodoStatusView::NotStarted),
        (TodoStatusInput::InProgress, in_progress),
        (TodoStatusInput::Done, done),
    ] {
        let todo = new_todo().into_any();
        let app = app_with(&todo).await;
        app.set_time(now);

        let moved = app
            .change_todo_status(UnvalidatedChangeTodoStatusInput {
                id: todo.id().into(),
                status,
            })
            .await
            .expect("active todo");

        assert_eq!(moved.status(), expected);
        let stored = app.store.find(todo.id()).await.expect("query");
        assert_eq!(stored.as_ref().map(TodoView::from), Some(moved));
    }
}

#[tokio::test]
async fn a_missing_todo_is_not_found() {
    let app = testing::app();

    let error = app
        .change_todo_status(UnvalidatedChangeTodoStatusInput {
            id: Uuid::from_u128(1),
            status: TodoStatusInput::Done,
        })
        .await
        .expect_err("no todo");

    assert!(matches!(
        error.describe(),
        ErrorInfo::NotFound(reason) if reason.code == "todo_not_found"
    ));
}

#[tokio::test]
async fn an_archived_todo_is_rejected() {
    let todo = new_todo().archive(Timestamp::UNIX_EPOCH).into_any();
    let app = app_with(&todo).await;

    let error = app
        .change_todo_status(UnvalidatedChangeTodoStatusInput {
            id: todo.id().into(),
            status: TodoStatusInput::Done,
        })
        .await
        .expect_err("archived todo");

    assert!(matches!(
        error.describe(),
        ErrorInfo::Conflict(reason) if reason.code == "todo_archived"
    ));
}
