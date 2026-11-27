use domain::assignee::Assignee;
use jiff::Timestamp;
use uuid::Uuid;

use super::AssigneeLifecycleView;
use super::AssigneeView;
use crate::app::pagination::ListQuery;
use crate::app::pagination::Page;

#[test]
fn active_and_deleted_views_preserve_the_recorded_fields() {
    let id = Uuid::from_u128(7);
    let created_at = Timestamp::new(1, 7).expect("valid creation time");
    let deleted_at = Timestamp::new(3, 1).expect("valid deletion time");
    let active = Assignee::new(id.into(), "Ada".parse().expect("valid name"), created_at);
    let active_view = AssigneeView::from(&active);
    let saved = active.delete(deleted_at).into_any();

    for (view, lifecycle) in [
        (active_view, AssigneeLifecycleView::Active),
        (
            AssigneeView::from(&saved),
            AssigneeLifecycleView::Deleted { deleted_at },
        ),
    ] {
        assert_eq!(view.id(), id);
        assert_eq!(view.name(), "Ada");
        assert_eq!(view.created_at(), created_at);
        assert_eq!(view.lifecycle(), lifecycle);
    }
}

#[test]
fn assignee_list_views_keep_the_query_order_and_continuation() {
    let [first, second, extra] = [(9, "Lin"), (3, "Grace"), (5, "Ada")].map(|(id, name)| {
        Assignee::new(
            Uuid::from_u128(id).into(),
            name.parse().expect("valid name"),
            Timestamp::UNIX_EPOCH,
        )
    });
    let expected_active = [AssigneeView::from(&first), AssigneeView::from(&second)];
    let deleted_at = Timestamp::new(3, 1).expect("valid deletion time");
    let deleted = second.clone().delete(deleted_at);
    let expected_deleted = [AssigneeView::from(&deleted)];
    let query = ListQuery::try_new(2, 4).expect("valid page");
    let active = Page::from_lookahead(query, vec![first, second, extra]).expect("valid page");

    let active_views = Page::<AssigneeView>::from(active);
    assert_eq!(active_views.items(), expected_active);
    assert_eq!(active_views.next_offset(), Some(6));

    let deleted_views = Page::<AssigneeView>::from(Page::complete(vec![deleted]));
    assert_eq!(deleted_views.items(), expected_deleted);
    assert_eq!(deleted_views.next_offset(), None);
}
