use std::iter;

use domain::assignee::Assignee;
use jiff::Timestamp;
use uuid::Uuid;

use crate::error::AppError;
use crate::error::ErrorInfo;
use crate::port::PortError;
use crate::port::assignee::Repository;
use crate::port::todo::Query;
use crate::testing;
use crate::todo::create::CreateTodoCommand;
use crate::todo::create::CreateTodoError;
use crate::todo::create::UnvalidatedCreateTodoInput;
use crate::todo::view::TodoLifecycleView;
use crate::todo::view::TodoStatusView;
use crate::todo::view::TodoView;

fn input(title: &str, description: Option<&str>) -> UnvalidatedCreateTodoInput {
    UnvalidatedCreateTodoInput {
        title: title.into(),
        description: description.map(str::to_owned),
        assignee_id: None,
        due_at: None,
    }
}

fn violations(error: &CreateTodoError) -> Vec<(&'static str, &'static str)> {
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
    let title = ("title", "blank");
    let description = ("description", "invalid_character");
    for (raw, expected) in [
        (input("", None), vec![title]),
        (input("title", Some("a\0b")), vec![description]),
        (input("", Some("a\0b")), vec![title, description]),
    ] {
        let error = CreateTodoCommand::try_from(raw).expect_err("invalid input");
        assert_eq!(violations(&error), expected);
    }
}

#[test]
fn a_storage_failure_is_described_by_the_port() {
    for (port, expected) in [
        (PortError::Busy("locked".into()), ErrorInfo::Unavailable),
        (PortError::Backend("corrupt".into()), ErrorInfo::Internal),
    ] {
        assert_eq!(CreateTodoError::from(port).describe(), expected);
    }
}

#[tokio::test]
async fn a_created_todo_is_stored_active_and_not_started_with_the_given_values() {
    let app = testing::app();
    let now = Timestamp::from_second(1_000).expect("valid time");
    app.set_time(now);
    let assignee = Assignee::new(
        Uuid::from_u128(7).into(),
        "Ada".parse().expect("valid name"),
        Timestamp::UNIX_EPOCH,
    )
    .into_any();
    app.transact(async |uow| uow.save(&assignee).await)
        .await
        .expect("stored assignee");
    let due_at = Timestamp::from_second(2_000).expect("valid deadline");

    let created = app
        .create_todo(UnvalidatedCreateTodoInput {
            assignee_id: Some(assignee.id().into()),
            due_at: Some(due_at),
            ..input("Review design", Some("notes"))
        })
        .await
        .expect("valid todo");

    assert_eq!(created.id(), Uuid::from_u128(1));
    assert_eq!(created.title(), "Review design");
    assert_eq!(created.description(), Some("notes"));
    assert_eq!(created.assignee_id(), Some(assignee.id().into()));
    assert_eq!(created.due_at(), Some(due_at));
    assert_eq!(created.created_at(), now);
    assert_eq!(created.status(), TodoStatusView::NotStarted);
    assert_eq!(created.lifecycle(), TodoLifecycleView::Active);
    let stored = app.store.find(created.id().into()).await.expect("query");
    assert_eq!(stored.as_ref().map(TodoView::from), Some(created));
}

#[tokio::test]
async fn a_blank_description_is_accepted_as_none() {
    let app = testing::app();

    let created = app
        .create_todo(input("title", Some(" \t ")))
        .await
        .expect("valid todo");

    assert_eq!(created.description(), None);
}

#[tokio::test]
async fn an_unknown_assignee_is_rejected() {
    let app = testing::app();

    let error = app
        .create_todo(UnvalidatedCreateTodoInput {
            assignee_id: Some(Uuid::from_u128(99)),
            ..input("title", None)
        })
        .await
        .expect_err("no active assignee");

    assert_eq!(violations(&error), [("assignee_id", "not_found")]);
}
