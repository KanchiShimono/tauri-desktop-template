use jiff::Timestamp;
use uuid::Uuid;

use super::AssigneeRow;
use crate::domain::assignee::Assignee;
use crate::sqlite::test_db;

#[sqlx::test]
async fn active_and_deleted_assignees_round_trip() {
    let mut db = test_db::migrated().await;
    let active = Assignee::new(
        Uuid::max().into(),
        "Ada".parse().expect("valid name"),
        Timestamp::UNIX_EPOCH,
    );
    let deleted_at = Timestamp::new(1, 123).expect("valid timestamp");
    for (assignee, deleted_at) in [
        (active.clone().into_any(), None),
        (active.delete(deleted_at).into_any(), Some(deleted_at)),
    ] {
        let row: AssigneeRow = (&assignee).into();
        assert_eq!(row.deleted_at, deleted_at.map(Into::into));
        let stored: AssigneeRow = sqlx::query_as(
            "INSERT OR REPLACE INTO assignees (id, name, created_at, deleted_at)
             VALUES (?, ?, ?, ?) RETURNING *",
        )
        .bind(row.id)
        .bind(row.name)
        .bind(row.created_at)
        .bind(row.deleted_at)
        .fetch_one(&mut db)
        .await
        .expect("read stored assignee");
        let restored: Assignee = stored.try_into().expect("valid assignee row");
        assert_eq!(restored, assignee);
    }
}
