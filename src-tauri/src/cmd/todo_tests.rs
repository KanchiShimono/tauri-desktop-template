use app::clock::SystemClock;
use app::id::UuidV7Generator;
use app::pagination::ListQuery;
use app::pagination::Page;
use app::pagination::UnvalidatedListQuery;
use app::presence::Presence;
use app::todo::change_status::TodoStatusInput;
use app::todo::change_status::UnvalidatedChangeTodoStatusInput;
use app::todo::create::UnvalidatedCreateTodoInput;
use app::todo::edit::UnvalidatedEditTodoInput;
use app::todo::view::TodoStatusView;
use app::todo::view::TodoView;
use jiff::Timestamp;
use serde_json::Value;
use serde_json::json;
use uuid::Uuid;

use super::ChangeTodoStatusRequest;
use super::CreateTodoRequest;
use super::EditTodoRequest;
use super::ListActiveTodosResponse;
use super::ListArchivedTodosRequest;
use super::ListArchivedTodosResponse;
use super::TodoStatus;
use crate::App;
use crate::sqlite::test_db;

fn edit(request: Value) -> UnvalidatedEditTodoInput {
    serde_json::from_value::<EditTodoRequest>(request)
        .expect("valid request")
        .into()
}

async fn views(count: usize) -> Vec<TodoView> {
    let app = App::new(test_db::store().await, SystemClock, UuidV7Generator);
    let mut views = Vec::with_capacity(count);
    for number in 0..count {
        let input = UnvalidatedCreateTodoInput {
            title: format!("Task {number}"),
            description: None,
            assignee_id: None,
            due_at: None,
        };
        views.push(app.create_todo(input).await.expect("created"));
    }
    views
}

#[test]
fn a_done_status_keeps_its_start_and_completion_apart() {
    let started_at = Timestamp::from_second(1).expect("valid time");
    let completed_at = Timestamp::from_second(2).expect("valid time");

    let status = TodoStatus::from(TodoStatusView::Done {
        started_at,
        completed_at,
    });

    assert_eq!(
        serde_json::to_value(status).expect("serializable"),
        json!({
            "kind": "done",
            "started_at": "1970-01-01T00:00:01Z",
            "completed_at": "1970-01-01T00:00:02Z",
        })
    );
}

#[test]
fn an_omitted_field_is_absent_and_a_null_field_is_cleared() {
    let id = Uuid::from_u128(1);

    let UnvalidatedEditTodoInput {
        id: _,
        title,
        description,
        assignee_id,
        due_at,
        started_at,
        completed_at,
    } = edit(json!({ "id": id }));
    assert!(title.is_absent());
    assert!(description.is_absent());
    assert!(assignee_id.is_absent());
    assert!(due_at.is_absent());
    assert!(started_at.is_absent());
    assert!(completed_at.is_absent());

    let cleared =
        edit(json!({ "id": id, "description": null, "assignee_id": null, "due_at": null }));
    assert_eq!(cleared.description, Presence::Present(None));
    assert_eq!(cleared.assignee_id, Presence::Present(None));
    assert_eq!(cleared.due_at, Presence::Present(None));

    let changed =
        edit(json!({ "id": id, "title": "Renamed", "started_at": "2026-11-27T00:00:00Z" }));
    assert_eq!(changed.title, Presence::Present("Renamed".to_owned()));
    assert!(changed.started_at.is_present());
}

#[test]
fn a_field_that_cannot_be_null_rejects_null() {
    for field in ["title", "started_at", "completed_at"] {
        let request = json!({ "id": Uuid::from_u128(1), field: null });

        let error = serde_json::from_value::<EditTodoRequest>(request).expect_err("rejected");

        assert!(error.is_data(), "{field}: {error}");
    }
}

#[test]
fn a_misspelled_optional_field_is_rejected_instead_of_ignored() {
    let creating =
        json!({ "title": "Task", "description": null, "assignee_id": null, "dueAt": null });
    let editing = json!({ "id": Uuid::from_u128(1), "dueAt": null });

    assert!(serde_json::from_value::<CreateTodoRequest>(creating).is_err());
    assert!(serde_json::from_value::<EditTodoRequest>(editing).is_err());
}

#[test]
fn the_status_is_named_in_snake_case() {
    for (name, expected) in [
        ("not_started", TodoStatusInput::NotStarted),
        ("in_progress", TodoStatusInput::InProgress),
        ("done", TodoStatusInput::Done),
    ] {
        let request = json!({ "id": Uuid::from_u128(1), "status": name });
        let request: ChangeTodoStatusRequest =
            serde_json::from_value(request).expect("valid request");

        let input = UnvalidatedChangeTodoStatusInput::from(request);

        assert_eq!(input.status, expected);
    }
}

#[test]
fn a_list_request_keeps_its_limit_and_offset() {
    let request = json!({ "limit": 20, "offset": 40 });
    let request: ListArchivedTodosRequest = serde_json::from_value(request).expect("valid request");

    let query = UnvalidatedListQuery::from(request);

    assert_eq!((query.limit, query.offset), (20, 40));
}

#[tokio::test]
async fn a_next_offset_beyond_the_wire_format_is_an_error() {
    let Ok(offset) = u32::MAX.try_into() else {
        return;
    };
    let query = ListQuery::try_new(1, offset).expect("valid query");
    let page = Page::from_lookahead(query, views(2).await).expect("page");

    assert!(ListActiveTodosResponse::try_from(&page).is_err());
    assert!(ListArchivedTodosResponse::try_from(&page).is_err());
}
