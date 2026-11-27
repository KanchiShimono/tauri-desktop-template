use domain::assignee::Assignee;
use jiff::Timestamp;
use uuid::Uuid;

use crate::assignee::view::AssigneeView;
use crate::pagination::UnvalidatedListQuery;
use crate::port::assignee::Repository;
use crate::testing;

#[tokio::test]
async fn the_requested_page_of_the_active_assignees_is_listed() {
    let assignees = [1, 2, 3, 4].map(|id| {
        Assignee::new(
            Uuid::from_u128(id).into(),
            "Ada".parse().expect("valid name"),
            Timestamp::UNIX_EPOCH,
        )
        .into_any()
    });
    let app = testing::app();
    for assignee in &assignees {
        app.transact(async |uow| uow.save(assignee).await)
            .await
            .expect("stored assignee");
    }

    let page = app
        .list_active_assignees(UnvalidatedListQuery {
            limit: 1,
            offset: 2,
        })
        .await
        .expect("valid query");

    assert_eq!(page.items(), [AssigneeView::from(&assignees[2])]);
    assert_eq!(page.next_offset(), Some(3));
}
