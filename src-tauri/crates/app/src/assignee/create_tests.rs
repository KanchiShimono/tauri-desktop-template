use jiff::Timestamp;
use uuid::Uuid;

use super::CreateAssigneeCommand;
use super::CreateAssigneeError;
use super::UnvalidatedCreateAssigneeInput;
use crate::app::assignee::view::AssigneeLifecycleView;
use crate::app::assignee::view::AssigneeView;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::port::PortError;
use crate::app::port::assignee::Query;
use crate::app::testing;

#[test]
fn an_invalid_name_is_reported_as_a_violation_of_the_name() {
    let error = CreateAssigneeCommand::try_from(UnvalidatedCreateAssigneeInput {
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
        assert_eq!(CreateAssigneeError::from(port).describe(), expected);
    }
}

#[tokio::test]
async fn a_created_assignee_is_stored_active_with_the_given_name() {
    let app = testing::app();
    let now = Timestamp::from_second(1_000).expect("valid time");
    app.set_time(now);

    let created = app
        .create_assignee(UnvalidatedCreateAssigneeInput { name: "Ada".into() })
        .await
        .expect("valid assignee");

    assert_eq!(created.id(), Uuid::from_u128(1));
    assert_eq!(created.name(), "Ada");
    assert_eq!(created.created_at(), now);
    assert_eq!(created.lifecycle(), AssigneeLifecycleView::Active);
    let stored = app.store.find(created.id().into()).await.expect("query");
    assert_eq!(stored.as_ref().map(AssigneeView::from), Some(created));
}
