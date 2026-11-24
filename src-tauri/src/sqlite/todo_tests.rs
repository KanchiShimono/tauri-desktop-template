use jiff::SignedDuration;
use jiff::Timestamp;
use uuid::Uuid;

use super::TodoRow;
use super::TodoRowError;
use crate::domain::assignee::Assignee;
use crate::domain::todo::Active;
use crate::domain::todo::NotStarted;
use crate::domain::todo::StatusState;
use crate::domain::todo::Todo;
use crate::sqlite::test_db;
use crate::sqlite::timestamp::SqliteTimestamp;

fn at(second: i64) -> Timestamp {
    Timestamp::new(second, 123_456_789).expect("valid timestamp")
}

fn new_todo() -> Todo<Active, NotStarted> {
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
    active: Todo<Active, S>,
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
async fn stored_todo_preserves_all_six_distinct_timestamps() {
    let mut db = test_db::migrated().await;
    let started_at = at(2);
    let nanosecond = SignedDuration::from_nanos(1);
    let completed_at = started_at - nanosecond;
    let due_at = started_at + nanosecond;
    let todo = new_todo()
        .change_due_at(Some(due_at.into()))
        .start(at(6).into())
        .complete(at(7).into())
        .change_started_at(started_at.into())
        .change_completed_at(completed_at.into())
        .archive(at(3))
        .delete(at(4))
        .into_any();
    let row = TodoRow::from(&todo);
    let stored: TodoRow = sqlx::query_as(
        "INSERT INTO todos (id, title, created_at, due_at, started_at, completed_at, archived_at, deleted_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING *",
    )
    .bind(row.id)
    .bind(row.title)
    .bind(row.created_at)
    .bind(row.due_at)
    .bind(row.started_at)
    .bind(row.completed_at)
    .bind(row.archived_at)
    .bind(row.deleted_at)
    .fetch_one(&mut db)
    .await
    .expect("read stored todo");
    assert_eq!(stored.due_at, Some(due_at.into()));
    assert_eq!(stored.started_at, Some(started_at.into()));
    assert_eq!(stored.completed_at, Some(completed_at.into()));
    assert_eq!(Todo::try_from(stored).expect("valid todo row"), todo);
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
