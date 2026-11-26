use jiff::Timestamp;
use uuid::Uuid;

use super::GetAssigneeError;
use crate::app::assignee::view::AssigneeView;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::port::PortError;
use crate::app::port::assignee::Repository;
use crate::app::testing;
use crate::domain::assignee::Assignee;

#[test]
fn a_storage_failure_is_described_by_the_port() {
    for (port, expected) in [
        (PortError::Busy("locked".into()), ErrorInfo::Unavailable),
        (PortError::Backend("corrupt".into()), ErrorInfo::Internal),
    ] {
        assert_eq!(GetAssigneeError::from(port).describe(), expected);
    }
}

#[tokio::test]
async fn a_deleted_assignee_is_still_returned() {
    let assignee = Assignee::new(
        Uuid::from_u128(1).into(),
        "Ada".parse().expect("valid name"),
        Timestamp::UNIX_EPOCH,
    )
    .delete(Timestamp::UNIX_EPOCH)
    .into_any();
    let app = testing::app();
    app.transact(async |uow| uow.save(&assignee).await)
        .await
        .expect("stored assignee");

    let found = app
        .get_assignee(assignee.id().into())
        .await
        .expect("stored assignee");

    assert_eq!(found, AssigneeView::from(&assignee));
}

#[tokio::test]
async fn a_missing_assignee_is_not_found() {
    let app = testing::app();

    let error = app
        .get_assignee(Uuid::from_u128(1))
        .await
        .expect_err("no assignee");

    assert!(matches!(
        error.describe(),
        ErrorInfo::NotFound(reason) if reason.code == "assignee_not_found"
    ));
}
