use jiff::Timestamp;
use uuid::Uuid;

use super::AssigneeRow;
use super::AssigneeRowError;
use crate::app::pagination::ListQuery;
use crate::app::port::PortError;
use crate::app::port::UnitOfWork;
use crate::app::port::UnitOfWorkFactory;
use crate::app::port::assignee::Query;
use crate::app::port::assignee::Repository;
use crate::domain::assignee::Active;
use crate::domain::assignee::Assignee;
use crate::sqlite::store::Store;
use crate::sqlite::test_db;

fn at(second: i64) -> Timestamp {
    Timestamp::from_second(second).expect("valid timestamp")
}

fn new_assignee(id: u128, created_at: Timestamp) -> Assignee<Active> {
    Assignee::new(
        Uuid::from_u128(id).into(),
        "Ada".parse().expect("valid name"),
        created_at,
    )
}

async fn save(store: &Store, assignee: &Assignee) -> Assignee {
    let mut uow = store.begin().await.expect("begin");
    let saved = uow.save(assignee).await.expect("save");
    uow.commit().await.expect("commit");
    saved
}

#[test]
fn a_row_outside_the_queried_lifecycle_is_a_backend_failure() {
    let deleted = new_assignee(1, at(0)).delete(at(1)).into_any();
    let result = Assignee::<Active>::try_from(AssigneeRow::from(&deleted));
    let error = result.expect_err("a deleted row is not active");
    assert!(matches!(error, AssigneeRowError::UnexpectedLifecycle));
    assert!(matches!(PortError::from(error), PortError::Backend(_)));
}

#[sqlx::test]
async fn a_saved_assignee_is_found_by_its_id_and_saving_again_replaces_every_column() {
    let store = test_db::store().await;
    let grace = "Grace".parse().expect("valid name");
    let saves = [
        new_assignee(1, at(1))
            .rename(grace)
            .delete(at(2))
            .into_any(),
        new_assignee(1, at(0)).into_any(),
    ];

    for assignee in saves {
        assert_eq!(save(&store, &assignee).await, assignee);
        let found = store.find(assignee.id()).await.expect("query");
        assert_eq!(found, Some(assignee));
    }
    let other = Uuid::from_u128(2).into();
    assert_eq!(store.find(other).await.expect("query"), None);
}

#[sqlx::test]
async fn only_an_active_assignee_is_loaded() {
    let store = test_db::store().await;
    let active = new_assignee(1, at(0));
    let deleted = new_assignee(2, at(0)).delete(at(1));
    save(&store, &active.clone().into_any()).await;
    save(&store, &deleted.clone().into_any()).await;

    let mut uow = store.begin().await.expect("begin");
    let loaded = uow.load_active(active.id()).await.expect("load");
    assert_eq!(loaded, Some(active));
    assert_eq!(uow.load_active(deleted.id()).await.expect("load"), None);
}

#[sqlx::test]
async fn active_assignees_are_paged_in_creation_order_with_the_id_as_tie_breaker() {
    let store = test_db::store().await;
    let ordered =
        [(3, at(1)), (1, at(2)), (2, at(2))].map(|(id, created_at)| new_assignee(id, created_at));
    for assignee in ordered.iter().rev() {
        save(&store, &assignee.clone().into_any()).await;
    }
    save(&store, &new_assignee(4, at(0)).delete(at(1)).into_any()).await;

    for (limit, offset, range, next_offset) in [(2, 0, 0..2, Some(2)), (2, 2, 2..3, None)] {
        let query = ListQuery::try_new(limit, offset).expect("valid query");
        let page = store.list_active(query).await.expect("query");
        assert_eq!(
            (page.items(), page.next_offset()),
            (&ordered[range], next_offset),
            "limit {limit}, offset {offset}"
        );
    }
}
