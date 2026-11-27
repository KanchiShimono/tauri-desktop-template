use domain::assignee;
use domain::assignee::Assignee;
use domain::todo::Active;
use domain::todo::NotStarted;
use domain::todo::Todo;
use jiff::Timestamp;
use uuid::Uuid;

use super::AssignedTodoPolicy;
use super::DeleteAssigneeError;
use super::UnvalidatedDeleteAssigneeInput;
use crate::app::assignee::view::AssigneeLifecycleView;
use crate::app::assignee::view::AssigneeView;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::port;
use crate::app::port::PortError;
use crate::app::testing;

fn new_assignee(id: u128) -> Assignee<assignee::Active> {
    Assignee::new(
        Uuid::from_u128(id).into(),
        "Ada".parse().expect("valid name"),
        Timestamp::UNIX_EPOCH,
    )
}

fn new_todo(id: u128) -> Todo<Active, NotStarted> {
    Todo::new(
        Uuid::from_u128(id).into(),
        "Review design".parse().expect("valid title"),
        None,
        None,
        None,
        Timestamp::UNIX_EPOCH,
    )
}

#[test]
fn a_storage_failure_is_described_by_the_port() {
    for (port, expected) in [
        (PortError::Busy("locked".into()), ErrorInfo::Unavailable),
        (PortError::Backend("corrupt".into()), ErrorInfo::Internal),
    ] {
        assert_eq!(DeleteAssigneeError::from(port).describe(), expected);
    }
}

#[tokio::test]
async fn an_assignee_is_deleted_at_the_current_time_and_unassigned_from_every_todo() {
    let app = testing::app();
    let now = Timestamp::from_second(1_000).expect("valid time");
    app.set_time(now);
    let ada = new_assignee(1);
    let active = new_todo(2).reassign(&ada).into_any();
    let archived = new_todo(3)
        .reassign(&ada)
        .archive(Timestamp::UNIX_EPOCH)
        .into_any();
    let ada = ada.into_any();
    app.transact(async |uow| {
        port::assignee::Repository::save(uow, &ada).await?;
        port::todo::Repository::save(uow, &active).await?;
        port::todo::Repository::save(uow, &archived).await
    })
    .await
    .expect("stored");

    let deleted = app
        .delete_assignee(UnvalidatedDeleteAssigneeInput {
            id: ada.id().into(),
            assigned_todo_policy: AssignedTodoPolicy::Unassign,
        })
        .await
        .expect("active assignee");

    assert_eq!(
        deleted.lifecycle(),
        AssigneeLifecycleView::Deleted { deleted_at: now }
    );
    let stored = port::assignee::Query::find(&app.store, ada.id())
        .await
        .expect("query");
    assert_eq!(stored.as_ref().map(AssigneeView::from), Some(deleted));
    for todo in [active, archived] {
        let stored = port::todo::Query::find(&app.store, todo.id())
            .await
            .expect("query")
            .expect("stored todo");
        assert_eq!(stored.assignee_id(), None);
    }
}

#[tokio::test]
async fn todos_that_are_only_deleted_do_not_block_deletion_but_lose_their_assignee() {
    let app = testing::app();
    let ada = new_assignee(1);
    let deleted = new_todo(2)
        .reassign(&ada)
        .delete(Timestamp::UNIX_EPOCH)
        .into_any();
    let ada = ada.into_any();
    app.transact(async |uow| {
        port::assignee::Repository::save(uow, &ada).await?;
        port::todo::Repository::save(uow, &deleted).await
    })
    .await
    .expect("stored");

    app.delete_assignee(UnvalidatedDeleteAssigneeInput {
        id: ada.id().into(),
        assigned_todo_policy: AssignedTodoPolicy::Reject,
    })
    .await
    .expect("no undeleted todos");

    let stored = port::todo::Query::find(&app.store, deleted.id())
        .await
        .expect("query")
        .expect("stored todo");
    assert_eq!(stored.assignee_id(), None);
}

#[tokio::test]
async fn a_missing_assignee_is_not_found() {
    let app = testing::app();

    let error = app
        .delete_assignee(UnvalidatedDeleteAssigneeInput {
            id: Uuid::from_u128(1),
            assigned_todo_policy: AssignedTodoPolicy::Reject,
        })
        .await
        .expect_err("no assignee");

    assert!(matches!(
        error.describe(),
        ErrorInfo::NotFound(reason) if reason.code == "assignee_not_found"
    ));
}

#[tokio::test]
async fn an_undeleted_todo_blocks_deletion_when_assigned_todos_are_rejected() {
    let app = testing::app();
    let ada = new_assignee(1);
    let archived = new_todo(2)
        .reassign(&ada)
        .archive(Timestamp::UNIX_EPOCH)
        .into_any();
    let ada = ada.into_any();
    app.transact(async |uow| {
        port::assignee::Repository::save(uow, &ada).await?;
        port::todo::Repository::save(uow, &archived).await
    })
    .await
    .expect("stored");

    let error = app
        .delete_assignee(UnvalidatedDeleteAssigneeInput {
            id: ada.id().into(),
            assigned_todo_policy: AssignedTodoPolicy::Reject,
        })
        .await
        .expect_err("undeleted todo");

    assert!(matches!(
        error.describe(),
        ErrorInfo::Conflict(reason) if reason.code == "assignee_in_use"
    ));
}
