use domain::assignee::Assignee;
use jiff::Timestamp;
use uuid::Uuid;

use super::RenameAssigneeCommand;
use super::RenameAssigneeError;
use super::UnvalidatedRenameAssigneeInput;
use crate::app::assignee::view::AssigneeView;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::port::PortError;
use crate::app::port::assignee::Query;
use crate::app::port::assignee::Repository;
use crate::app::testing;

#[test]
fn an_invalid_name_is_reported_as_a_violation_of_the_name() {
    let error = RenameAssigneeCommand::try_from(UnvalidatedRenameAssigneeInput {
        id: Uuid::nil(),
        name: String::new(),
    })
    .expect_err("invalid name");

    assert!(matches!(
        error.describe(),
        ErrorInfo::InvalidInput(violations)
            if violations.first.field == "name" && violations.first.reason.code == "blank"
    ));
}

#[test]
fn a_storage_failure_is_described_by_the_port() {
    for (port, expected) in [
        (PortError::Busy("locked".into()), ErrorInfo::Unavailable),
        (PortError::Backend("corrupt".into()), ErrorInfo::Internal),
    ] {
        assert_eq!(RenameAssigneeError::from(port).describe(), expected);
    }
}

#[tokio::test]
async fn an_active_assignee_is_renamed() {
    let assignee = Assignee::new(
        Uuid::from_u128(1).into(),
        "Ada".parse().expect("valid name"),
        Timestamp::UNIX_EPOCH,
    )
    .into_any();
    let app = testing::app();
    app.transact(async |uow| uow.save(&assignee).await)
        .await
        .expect("stored assignee");

    let renamed = app
        .rename_assignee(UnvalidatedRenameAssigneeInput {
            id: assignee.id().into(),
            name: "Grace".into(),
        })
        .await
        .expect("active assignee");

    assert_eq!(renamed.name(), "Grace");
    let stored = app.store.find(assignee.id()).await.expect("query");
    assert_eq!(stored.as_ref().map(AssigneeView::from), Some(renamed));
}

#[tokio::test]
async fn a_missing_assignee_is_not_found() {
    let app = testing::app();

    let error = app
        .rename_assignee(UnvalidatedRenameAssigneeInput {
            id: Uuid::from_u128(1),
            name: "Grace".into(),
        })
        .await
        .expect_err("no assignee");

    assert!(matches!(
        error.describe(),
        ErrorInfo::NotFound(reason) if reason.code == "assignee_not_found"
    ));
}
