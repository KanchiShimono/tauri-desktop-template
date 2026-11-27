use serde_json::json;
use uuid::Uuid;

use super::DeleteAssigneeRequest;
use super::ListActiveAssigneesRequest;
use super::ListActiveAssigneesResponse;
use crate::App;
use crate::app::AssigneeView;
use crate::app::ListQuery;
use crate::app::Page;
use crate::app::SystemClock;
use crate::app::UuidV7Generator;
use crate::app::assignee::create::UnvalidatedCreateAssigneeInput;
use crate::app::assignee::delete::AssignedTodoPolicy;
use crate::app::assignee::delete::UnvalidatedDeleteAssigneeInput;
use crate::app::pagination::UnvalidatedListQuery;
use crate::sqlite::test_db;

async fn views(count: usize) -> Vec<AssigneeView> {
    let app = App::new(test_db::store().await, SystemClock, UuidV7Generator);
    let mut views = Vec::with_capacity(count);
    for number in 0..count {
        let input = UnvalidatedCreateAssigneeInput {
            name: format!("Assignee {number}"),
        };
        views.push(app.create_assignee(input).await.expect("created"));
    }
    views
}

#[test]
fn the_assigned_todo_policy_is_named_in_snake_case() {
    for (name, expected) in [
        ("reject", AssignedTodoPolicy::Reject),
        ("unassign", AssignedTodoPolicy::Unassign),
    ] {
        let request = json!({ "id": Uuid::from_u128(1), "assigned_todo_policy": name });
        let request: DeleteAssigneeRequest =
            serde_json::from_value(request).expect("valid request");

        let input = UnvalidatedDeleteAssigneeInput::from(request);

        assert_eq!(input.assigned_todo_policy, expected);
    }
}

#[test]
fn a_list_request_keeps_its_limit_and_offset() {
    let request = json!({ "limit": 20, "offset": 40 });
    let request: ListActiveAssigneesRequest =
        serde_json::from_value(request).expect("valid request");

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

    assert!(ListActiveAssigneesResponse::try_from(&page).is_err());
}
