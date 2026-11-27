use std::iter;

use domain::assignee::Assignee;
use domain::todo::Active;
use domain::todo::CompletedAt;
use domain::todo::NotStarted;
use domain::todo::StartedAt;
use domain::todo::StatusKind;
use domain::todo::Todo;
use jiff::Timestamp;
use uuid::Uuid;

use crate::error::AppError;
use crate::error::ErrorInfo;
use crate::port::PortError;
use crate::port::assignee;
use crate::port::todo;
use crate::port::todo::Query;
use crate::presence::Presence;
use crate::testing;
use crate::testing::TestApp;
use crate::todo::edit;
use crate::todo::edit::EditTodoCommand;
use crate::todo::edit::EditTodoError;
use crate::todo::edit::UnvalidatedEditTodoInput;
use crate::todo::view::TodoStatusView;
use crate::todo::view::TodoView;

fn at(second: i64) -> Timestamp {
    Timestamp::from_second(second).expect("valid time")
}

fn new_todo() -> Todo<Active, NotStarted> {
    Todo::new(
        Uuid::from_u128(1).into(),
        "Review design".parse().expect("valid title"),
        Some("notes".parse().expect("valid description")),
        Some(Uuid::from_u128(7).into()),
        Some(at(50).into()),
        Timestamp::UNIX_EPOCH,
    )
}

async fn app_with(todo: &Todo) -> TestApp {
    let app = testing::app();
    app.transact(async |uow| todo::Repository::save(uow, todo).await)
        .await
        .expect("stored todo");
    app
}

fn unchanged(id: Uuid) -> UnvalidatedEditTodoInput {
    UnvalidatedEditTodoInput {
        id,
        title: Presence::Absent,
        description: Presence::Absent,
        assignee_id: Presence::Absent,
        due_at: Presence::Absent,
        started_at: Presence::Absent,
        completed_at: Presence::Absent,
    }
}

fn violations(error: &EditTodoError) -> Vec<(&'static str, &'static str)> {
    let info = error.describe();
    let ErrorInfo::InvalidInput(violations) = info else {
        panic!("expected an input error, got {info:?}");
    };
    iter::once(violations.first)
        .chain(violations.rest)
        .map(|violation| (violation.field, violation.reason.code))
        .collect()
}

#[test]
fn every_invalid_field_is_reported_in_input_order() {
    let blank = Presence::Present(String::new());
    let nul = Presence::Present(Some("a\0b".to_owned()));
    let blank_title = ("title", "blank");
    let bad_description = ("description", "invalid_character");
    for (title, description, expected) in [
        (blank.clone(), Presence::Absent, vec![blank_title]),
        (Presence::Absent, nul.clone(), vec![bad_description]),
        (blank, nul, vec![blank_title, bad_description]),
    ] {
        let raw = UnvalidatedEditTodoInput {
            title,
            description,
            ..unchanged(Uuid::nil())
        };
        let error = EditTodoCommand::try_from(raw).expect_err("invalid input");
        assert_eq!(violations(&error), expected);
    }
}

#[test]
fn a_recorded_time_can_be_corrected_only_in_a_status_that_has_it() {
    let [not_started, in_progress] = [StatusKind::NotStarted, StatusKind::InProgress]
        .map(|status| new_todo().change_status(status, at(1)));
    let start = StartedAt::from(at(10));
    let completion = CompletedAt::from(at(20));
    let untouched = in_progress.clone().into_any();
    let started = new_todo().start(start).into_any();
    let no_start = Err(vec![("started_at", "not_started")]);
    let no_completion = Err(vec![("completed_at", "not_completed")]);

    for (todo, started_at, completed_at, expected) in [
        (not_started.clone(), Some(start), None, no_start),
        (not_started, None, Some(completion), no_completion.clone()),
        (in_progress.clone(), None, None, Ok(untouched)),
        (in_progress.clone(), Some(start), None, Ok(started)),
        (in_progress, None, Some(completion), no_completion),
    ] {
        let changed = edit::change_recorded_times(todo, started_at.into(), completed_at.into())
            .map_err(|error| violations(&error));
        assert_eq!(changed, expected);
    }
}

#[test]
fn a_storage_failure_is_described_by_the_port() {
    for (port, expected) in [
        (PortError::Busy("locked".into()), ErrorInfo::Unavailable),
        (PortError::Backend("corrupt".into()), ErrorInfo::Internal),
    ] {
        assert_eq!(EditTodoError::from(port).describe(), expected);
    }
}

#[tokio::test]
async fn every_present_field_replaces_the_stored_value() {
    let todo = new_todo()
        .start(at(1).into())
        .complete(at(2).into())
        .into_any();
    let app = app_with(&todo).await;
    let grace = Assignee::new(
        Uuid::from_u128(8).into(),
        "Grace".parse().expect("valid name"),
        Timestamp::UNIX_EPOCH,
    )
    .into_any();
    app.transact(async |uow| assignee::Repository::save(uow, &grace).await)
        .await
        .expect("stored assignee");

    let edited = app
        .edit_todo(UnvalidatedEditTodoInput {
            id: todo.id().into(),
            title: Presence::Present("Approve design".into()),
            description: Presence::Present(Some("approved".into())),
            assignee_id: Presence::Present(Some(grace.id().into())),
            due_at: Presence::Present(Some(at(60))),
            started_at: Presence::Present(at(10)),
            completed_at: Presence::Present(at(20)),
        })
        .await
        .expect("valid edit");

    assert_eq!(edited.title(), "Approve design");
    assert_eq!(edited.description(), Some("approved"));
    assert_eq!(edited.assignee_id(), Some(grace.id().into()));
    assert_eq!(edited.due_at(), Some(at(60)));
    assert_eq!(
        edited.status(),
        TodoStatusView::Done {
            started_at: at(10),
            completed_at: at(20),
        }
    );
    let stored = Query::find(&app.store, todo.id()).await.expect("query");
    assert_eq!(stored.as_ref().map(TodoView::from), Some(edited));
}

#[tokio::test]
async fn an_absent_field_keeps_its_value_and_a_present_none_clears_it() {
    let todo = new_todo()
        .start(at(1).into())
        .complete(at(2).into())
        .into_any();
    let app = app_with(&todo).await;

    let kept = app
        .edit_todo(unchanged(todo.id().into()))
        .await
        .expect("valid edit");
    assert_eq!(kept, TodoView::from(&todo));

    let cleared = app
        .edit_todo(UnvalidatedEditTodoInput {
            description: Presence::Present(None),
            assignee_id: Presence::Present(None),
            due_at: Presence::Present(None),
            ..unchanged(todo.id().into())
        })
        .await
        .expect("valid edit");
    assert_eq!(cleared.description(), None);
    assert_eq!(cleared.assignee_id(), None);
    assert_eq!(cleared.due_at(), None);
}

#[tokio::test]
async fn a_blank_description_clears_the_description() {
    let todo = new_todo().into_any();
    let app = app_with(&todo).await;

    let edited = app
        .edit_todo(UnvalidatedEditTodoInput {
            description: Presence::Present(Some(" \t ".into())),
            ..unchanged(todo.id().into())
        })
        .await
        .expect("valid edit");

    assert_eq!(edited.description(), None);
}

#[tokio::test]
async fn a_missing_todo_is_not_found() {
    let app = testing::app();

    let error = app
        .edit_todo(unchanged(Uuid::from_u128(1)))
        .await
        .expect_err("no todo");

    assert!(matches!(
        error.describe(),
        ErrorInfo::NotFound(reason) if reason.code == "todo_not_found"
    ));
}

#[tokio::test]
async fn an_archived_todo_is_rejected() {
    let todo = new_todo().archive(at(1)).into_any();
    let app = app_with(&todo).await;

    let error = app
        .edit_todo(unchanged(todo.id().into()))
        .await
        .expect_err("archived todo");

    assert!(matches!(
        error.describe(),
        ErrorInfo::Conflict(reason) if reason.code == "todo_archived"
    ));
}

#[tokio::test]
async fn an_unknown_assignee_is_rejected() {
    let todo = new_todo().into_any();
    let app = app_with(&todo).await;

    let error = app
        .edit_todo(UnvalidatedEditTodoInput {
            assignee_id: Presence::Present(Some(Uuid::from_u128(99))),
            ..unchanged(todo.id().into())
        })
        .await
        .expect_err("no active assignee");

    assert_eq!(violations(&error), [("assignee_id", "not_found")]);
}
