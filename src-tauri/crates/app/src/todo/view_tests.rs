use domain::todo::Active;
use domain::todo::NotStarted;
use domain::todo::StatusKind;
use domain::todo::Todo;
use jiff::Timestamp;
use uuid::Uuid;

use crate::pagination::ListQuery;
use crate::pagination::Page;
use crate::todo::view::PreviousTodoLifecycleView;
use crate::todo::view::TodoLifecycleView;
use crate::todo::view::TodoStatusView;
use crate::todo::view::TodoView;

fn new_todo(id: u128) -> Todo<Active, NotStarted> {
    Todo::new(
        Uuid::from_u128(id).into(),
        "Task".parse().expect("valid title"),
        None,
        None,
        None,
        Timestamp::UNIX_EPOCH,
    )
}

#[test]
fn a_saved_todo_preserves_each_optional_field_independently() {
    let id = Uuid::from_u128(8);
    let created_at = Timestamp::new(1, 8).expect("valid creation time");
    let assignee_id = Uuid::from_u128(15);
    let due_at = Timestamp::new(42, 123).expect("valid deadline");
    let notes = Some("line one\n  line two");
    for (description, assignee, due) in [
        (None, None, None),
        (notes, None, None),
        (None, Some(assignee_id), None),
        (None, None, Some(due_at)),
        (notes, Some(assignee_id), Some(due_at)),
    ] {
        let saved = Todo::new(
            id.into(),
            "Review design".parse().expect("valid title"),
            description.map(|text| text.parse().expect("valid description")),
            assignee.map(Into::into),
            due.map(Into::into),
            created_at,
        )
        .into_any();
        let view = TodoView::from(&saved);
        assert_eq!(view.id(), id);
        assert_eq!(view.title(), "Review design");
        assert_eq!(view.description(), description);
        assert_eq!(view.assignee_id(), assignee);
        assert_eq!(view.due_at(), due);
        assert_eq!(view.created_at(), created_at);
        assert_eq!(view.status(), TodoStatusView::NotStarted);
        assert_eq!(view.lifecycle(), TodoLifecycleView::Active);
    }
}

#[test]
fn status_times_and_the_lifecycle_before_deletion_are_independent() {
    let started_at = Timestamp::new(1, 1).expect("valid start");
    let completed_at = Timestamp::new(2, 2).expect("valid completion");
    let archived_at = Timestamp::new(3, 3).expect("valid archive time");
    let deleted_at = Timestamp::new(4, 4).expect("valid deletion time");
    let started = new_todo(8).start(started_at.into());
    assert_eq!(
        TodoView::from(&started).status(),
        TodoStatusView::InProgress { started_at }
    );
    let done = started.complete(completed_at.into());
    let archived = done.clone().archive(archived_at);
    let done_status = TodoStatusView::Done {
        started_at,
        completed_at,
    };
    let view = TodoView::from(&archived);
    assert_eq!(
        view.lifecycle(),
        TodoLifecycleView::Archived { archived_at }
    );
    assert_eq!(view.status(), done_status);

    for (deleted, previous) in [
        (
            done.delete(deleted_at).into_any(),
            PreviousTodoLifecycleView::Active,
        ),
        (
            archived.delete(deleted_at).into_any(),
            PreviousTodoLifecycleView::Archived { archived_at },
        ),
    ] {
        let view = TodoView::from(&deleted);
        assert_eq!(view.status(), done_status);
        assert_eq!(
            view.lifecycle(),
            TodoLifecycleView::Deleted {
                deleted_at,
                previous
            }
        );
    }
}

#[test]
fn an_archived_page_preserves_the_query_order_and_continuation() {
    let started_at = Timestamp::new(1, 1).expect("valid start");
    let archived_at = Timestamp::new(3, 1).expect("valid archive time");
    let [first, second, extra] = [8, 4, 6].map(|id| {
        new_todo(id)
            .change_status(StatusKind::InProgress, started_at)
            .archive(archived_at)
    });
    let expected = [TodoView::from(&first), TodoView::from(&second)];
    let query = ListQuery::try_new(2, 6).expect("valid query");
    let page = Page::from_lookahead(query, vec![first, second, extra]).expect("valid page");

    let views = Page::<TodoView>::from(page);
    assert_eq!(views.items(), expected);
    assert_eq!(views.next_offset(), Some(8));
}

#[test]
fn an_active_query_result_becomes_one_complete_page_even_above_the_query_limit() {
    let statuses = [
        StatusKind::InProgress,
        StatusKind::Done,
        StatusKind::NotStarted,
    ];
    for count in [0, ListQuery::MAX_LIMIT + 1] {
        let rows: Vec<_> = (0..count as u128)
            .rev()
            .zip(statuses.into_iter().cycle())
            .map(|(id, status)| new_todo(id).change_status(status, Timestamp::UNIX_EPOCH))
            .collect();
        let expected: Vec<_> = rows.iter().map(TodoView::from).collect();
        let views = Page::<TodoView>::from(rows);
        assert_eq!(views.items().len(), count);
        assert_eq!(views.items(), expected);
        assert_eq!(views.next_offset(), None);
    }
}
