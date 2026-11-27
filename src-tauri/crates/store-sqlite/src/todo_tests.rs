use app::pagination::ListQuery;
use app::port;
use app::port::PortError;
use app::port::UnitOfWork;
use app::port::UnitOfWorkFactory;
use app::port::todo::Query;
use app::port::todo::Repository;
use domain::assignee;
use domain::assignee::Assignee;
use domain::todo;
use domain::todo::NotStarted;
use domain::todo::Status;
use domain::todo::StatusState;
use domain::todo::Todo;
use jiff::Timestamp;
use uuid::Uuid;

use crate::store::Store;
use crate::test_db;
use crate::timestamp::SqliteTimestamp;
use crate::todo::TodoRow;
use crate::todo::TodoRowError;

fn at(second: i64) -> Timestamp {
    Timestamp::new(second, 123_456_789).expect("valid timestamp")
}

fn new_todo() -> Todo<todo::Active, NotStarted> {
    Todo::new(
        Uuid::max().into(),
        "Review invoice".parse().expect("valid title"),
        None,
        None,
        None,
        at(0),
    )
}

fn assert_lifecycle_round_trips<S: StatusState>(
    active: Todo<todo::Active, S>,
    started_at: Option<Timestamp>,
    completed_at: Option<Timestamp>,
) {
    let archived_at = at(3);
    let deleted_at = at(4);
    let archived = active.clone().archive(archived_at);
    for (todo, archived_at, deleted_at) in [
        (active.clone().into_any(), None, None),
        (archived.clone().into_any(), Some(archived_at), None),
        (active.delete(deleted_at).into_any(), None, Some(deleted_at)),
        (
            archived.delete(deleted_at).into_any(),
            Some(archived_at),
            Some(deleted_at),
        ),
    ] {
        let row: TodoRow = (&todo).into();
        assert_eq!(row.started_at, started_at.map(Into::into));
        assert_eq!(row.completed_at, completed_at.map(Into::into));
        assert_eq!(row.archived_at, archived_at.map(Into::into));
        assert_eq!(row.deleted_at, deleted_at.map(Into::into));
        let restored: Todo = row.try_into().expect("valid todo row");
        assert_eq!(restored, todo);
    }
}

#[test]
fn each_status_round_trips_through_each_lifecycle() {
    let todo = new_todo();
    assert_lifecycle_round_trips(todo.clone(), None, None);

    // The domain preserves recorded timestamps even if the clock moved backwards.
    let started_at = at(2);
    let completed_at = at(1);
    let assignee = Assignee::new(
        Uuid::nil().into(),
        "Ada".parse().expect("valid name"),
        at(0),
    );
    let in_progress = todo
        .change_description(Some("  Check totals\n".parse().expect("valid description")))
        .reassign(&assignee)
        .change_due_at(Some(at(5).into()))
        .start(started_at.into());
    assert_lifecycle_round_trips(in_progress.clone(), Some(started_at), None);
    assert_lifecycle_round_trips(
        in_progress.complete(completed_at.into()),
        Some(started_at),
        Some(completed_at),
    );
}

#[sqlx::test]
async fn stored_timestamps_sort_and_filter_chronologically() {
    let mut db = test_db::migrated().await;
    let start = Timestamp::from_second(13).expect("13 seconds");
    let middle = Timestamp::new(13, 100_000_000).expect("13.1 seconds");
    let end = Timestamp::new(13, 120_000_000).expect("13.12 seconds");
    let times = [
        Timestamp::MIN,
        Timestamp::from_nanosecond(-1).expect("one nanosecond before the epoch"),
        Timestamp::UNIX_EPOCH,
        start,
        middle,
        end,
        Timestamp::MAX,
    ]
    .map(SqliteTimestamp::from);
    for (index, time) in times.into_iter().enumerate().rev() {
        sqlx::query(
            "INSERT INTO todos (id, title, created_at, archived_at) VALUES (?1, 'Test', ?2, ?2)",
        )
        .bind(Uuid::from_u128(index as u128).hyphenated())
        .bind(time)
        .execute(&mut db)
        .await
        .expect("insert timestamp");
    }

    let ordered: Vec<SqliteTimestamp> =
        sqlx::query_scalar("SELECT created_at FROM todos ORDER BY created_at")
            .fetch_all(&mut db)
            .await
            .expect("sort stored timestamps");
    assert_eq!(ordered, times);

    let selected: Vec<SqliteTimestamp> = sqlx::query_scalar(
        "SELECT archived_at FROM todos
         WHERE deleted_at IS NULL AND archived_at >= ? AND archived_at < ?
         ORDER BY archived_at",
    )
    .bind(SqliteTimestamp::from(start))
    .bind(SqliteTimestamp::from(end))
    .fetch_all(&mut db)
    .await
    .expect("select half-open range");
    assert_eq!(selected, [start, middle].map(SqliteTimestamp::from));
}

#[test]
fn blank_stored_description_round_trips_as_null() {
    let mut row: TodoRow = (&new_todo().into_any()).into();
    row.description = Some(" \t\n".into());
    let restored: Todo = row.try_into().expect("blank description is optional");
    assert!(restored.description().is_none());
    let stored: TodoRow = (&restored).into();
    assert_eq!(stored.description, None);
}

#[test]
fn completion_requires_a_start_time() {
    let mut row: TodoRow = (&new_todo().into_any()).into();
    row.completed_at = Some(at(2).into());
    let result: Result<Todo, _> = row.try_into();
    assert!(matches!(result, Err(TodoRowError::CompletedWithoutStart)));
}

#[test]
fn a_row_outside_the_queried_lifecycle_is_a_backend_failure() {
    let deleted = new_todo().delete(at(1)).into_any();
    let result = Todo::<todo::Active, Status>::try_from(TodoRow::from(&deleted));
    let error = result.expect_err("a deleted row is not active");
    assert!(matches!(error, TodoRowError::UnexpectedLifecycle));
    assert!(matches!(PortError::from(error), PortError::Backend(_)));
}

fn new_assignee(id: u128) -> Assignee<assignee::Active> {
    Assignee::new(
        Uuid::from_u128(id).into(),
        "Ada".parse().expect("valid name"),
        at(0),
    )
}

fn todo_created_at(id: u128, created_at: Timestamp) -> Todo<todo::Active, NotStarted> {
    Todo::new(
        Uuid::from_u128(id).into(),
        "Review invoice".parse().expect("valid title"),
        None,
        None,
        None,
        created_at,
    )
}

async fn save_assignee(store: &Store, assignee: &Assignee<assignee::Active>) {
    let mut uow = store.begin().await.expect("begin");
    port::assignee::Repository::save(&mut uow, &assignee.clone().into_any())
        .await
        .expect("save assignee");
    uow.commit().await.expect("commit");
}

async fn save(store: &Store, todo: &Todo) -> Todo {
    let mut uow = store.begin().await.expect("begin");
    let saved = uow.save(todo).await.expect("save");
    uow.commit().await.expect("commit");
    saved
}

#[sqlx::test]
async fn a_saved_todo_is_found_by_its_id_and_saving_again_replaces_every_column() {
    let store = test_db::store().await;
    let ada = new_assignee(100);
    save_assignee(&store, &ada).await;
    let saves = [
        todo_created_at(1, at(6))
            .rename("Approve invoice".parse().expect("valid title"))
            .change_description(Some("Check totals".parse().expect("valid description")))
            .reassign(&ada)
            .change_due_at(Some(at(5).into()))
            .start(at(1).into())
            .complete(at(2).into())
            .archive(at(3))
            .delete(at(4))
            .into_any(),
        todo_created_at(1, at(0)).into_any(),
    ];

    for todo in saves {
        assert_eq!(save(&store, &todo).await, todo);
        assert_eq!(store.find(todo.id()).await.expect("query"), Some(todo));
    }
    let other = Uuid::from_u128(2).into();
    assert_eq!(store.find(other).await.expect("query"), None);
}

#[sqlx::test]
async fn only_an_undeleted_todo_is_loaded() {
    let store = test_db::store().await;
    let active = todo_created_at(1, at(0)).into_any();
    let archived = todo_created_at(2, at(0)).archive(at(1)).into_any();
    let deleted = todo_created_at(3, at(0)).delete(at(1)).into_any();
    for todo in [&active, &archived, &deleted] {
        save(&store, todo).await;
    }

    let mut uow = store.begin().await.expect("begin");
    for todo in [active, archived] {
        let loaded = uow.load_undeleted(todo.id()).await.expect("load");
        assert_eq!(loaded.map(Todo::into_any), Some(todo));
    }
    assert_eq!(uow.load_undeleted(deleted.id()).await.expect("load"), None);
}

#[sqlx::test]
async fn active_todos_are_listed_in_creation_order() {
    let store = test_db::store().await;
    let ordered =
        [(2, at(1)), (1, at(2))].map(|(id, created_at)| todo_created_at(id, created_at).into_any());
    for todo in ordered.iter().rev() {
        save(&store, todo).await;
    }
    save(&store, &todo_created_at(3, at(0)).archive(at(1)).into_any()).await;
    save(&store, &todo_created_at(4, at(0)).delete(at(1)).into_any()).await;

    let listed = store.list_active().await.expect("query");
    let listed = listed.into_iter().map(Todo::into_any).collect::<Vec<_>>();
    assert_eq!(listed, ordered);
}

#[sqlx::test]
async fn archived_todos_are_paged_from_the_latest_archive_with_the_id_as_tie_breaker() {
    let store = test_db::store().await;
    let ordered = [(1, at(9)), (3, at(8)), (2, at(8))]
        .map(|(id, archived_at)| todo_created_at(id, at(0)).archive(archived_at).into_any());
    for todo in &ordered {
        save(&store, todo).await;
    }
    save(&store, &todo_created_at(4, at(0)).into_any()).await;
    let deleted = todo_created_at(5, at(0)).archive(at(10)).delete(at(11));
    save(&store, &deleted.into_any()).await;

    for (limit, offset, range, next_offset) in [(2, 0, 0..2, Some(2)), (2, 2, 2..3, None)] {
        let query = ListQuery::try_new(limit, offset).expect("valid query");
        let page = store.list_archived(query).await.expect("query");
        let items = page
            .items()
            .iter()
            .cloned()
            .map(Todo::into_any)
            .collect::<Vec<_>>();
        assert_eq!(
            (items.as_slice(), page.next_offset()),
            (&ordered[range], next_offset),
            "limit {limit}, offset {offset}"
        );
    }
}

#[sqlx::test]
async fn the_todos_of_an_assignee_are_loaded_in_any_lifecycle_and_checked_for_an_undeleted_one() {
    let store = test_db::store().await;
    let [ada, grace] = [100, 101].map(new_assignee);
    for assignee in [&ada, &grace] {
        save_assignee(&store, assignee).await;
    }
    let of_ada = [
        todo_created_at(1, at(1))
            .reassign(&ada)
            .archive(at(3))
            .into_any(),
        todo_created_at(2, at(2))
            .reassign(&ada)
            .delete(at(3))
            .into_any(),
    ];
    let of_grace = todo_created_at(3, at(0))
        .reassign(&grace)
        .delete(at(3))
        .into_any();
    for todo in of_ada.iter().chain([&of_grace]) {
        save(&store, todo).await;
    }

    let mut uow = store.begin().await.expect("begin");
    assert_eq!(uow.load_all_for(ada.id()).await.expect("load"), of_ada);
    assert!(uow.has_undeleted_for(ada.id()).await.expect("query"));
    assert!(!uow.has_undeleted_for(grace.id()).await.expect("query"));
}
