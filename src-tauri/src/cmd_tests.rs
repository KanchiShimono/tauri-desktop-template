use std::env;
use std::fs;
use std::process;

use app::clock::SystemClock;
use app::id::UuidV7Generator;
use serde_json::Value;
use serde_json::json;
use specta_typescript::Typescript;
use store_sqlite::test_db;
use tauri::Manager;
use tauri::WebviewWindow;
use tauri::WebviewWindowBuilder;
use tauri::Wry;
use tauri::async_runtime;
use tauri::ipc::CallbackFn;
use tauri::ipc::InvokeBody;
use tauri::test;
use tauri::test::INVOKE_KEY;
use tauri::test::MockRuntime;
use tauri::webview::InvokeRequest;

use crate::App;

const BINDINGS: &str = "../src/bindings.gen.ts";

#[test]
fn bindings_are_up_to_date() {
    let exported = env::temp_dir().join(format!("kanban-bindings-{}.ts", process::id()));
    super::builder::<Wry>()
        .export(Typescript::default(), &exported)
        .expect("export bindings");
    let generated = fs::read_to_string(&exported).expect("read exported bindings");
    fs::remove_file(&exported).expect("remove exported bindings");

    if env::var_os("UPDATE_BINDINGS").is_some() {
        fs::write(BINDINGS, &generated).expect("write bindings");
    }

    let committed = fs::read_to_string(BINDINGS).unwrap_or_default();
    assert!(
        committed == generated,
        "{BINDINGS} is stale; run `UPDATE_BINDINGS=1 cargo test bindings_are_up_to_date`"
    );
}

fn window() -> WebviewWindow<MockRuntime> {
    let store = async_runtime::block_on(test_db::store());
    let app = test::mock_builder()
        .invoke_handler(super::builder().invoke_handler())
        .build(test::mock_context(test::noop_assets()))
        .expect("build app");
    app.manage(App::new(store, SystemClock, UuidV7Generator));
    WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("build window")
}

fn invoke(window: &WebviewWindow<MockRuntime>, cmd: &str, args: Value) -> Result<Value, Value> {
    let request = InvokeRequest {
        cmd: cmd.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: "tauri://localhost".parse().expect("valid url"),
        body: InvokeBody::Json(args),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_owned(),
    };
    test::get_ipc_response(window, request).map(|body| body.deserialize().expect("JSON response"))
}

#[test]
fn a_todo_round_trips_through_ipc() {
    let window = window();

    let created = invoke(
        &window,
        "create_todo",
        json!({ "request": {
            "title": " Write the design ",
            "description": null,
            "assignee_id": null,
            "due_at": "2026-12-01T00:00:00+09:00",
        } }),
    )
    .expect("created");
    let todo = &created["todo"];
    assert_eq!(todo["title"], "Write the design");
    assert_eq!(todo["due_at"], "2026-11-30T15:00:00Z");
    assert_eq!(todo["status"], json!({ "kind": "not_started" }));
    assert_eq!(todo["lifecycle"], json!({ "kind": "active" }));

    let moved = invoke(
        &window,
        "change_todo_status",
        json!({ "request": { "id": todo["id"], "status": "in_progress" } }),
    )
    .expect("moved");
    assert_eq!(moved["todo"]["status"]["kind"], "in_progress");

    let edited = invoke(
        &window,
        "edit_todo",
        json!({ "request": { "id": todo["id"], "due_at": null } }),
    )
    .expect("edited");
    assert_eq!(edited["todo"]["title"], "Write the design");
    assert_eq!(edited["todo"]["due_at"], Value::Null);
    assert_eq!(edited["todo"]["status"], moved["todo"]["status"]);

    let listed = invoke(&window, "list_active_todos", json!({})).expect("listed");
    assert_eq!(
        listed,
        json!({ "items": [edited["todo"]], "next_offset": null })
    );

    let id = json!({ "request": { "id": todo["id"] } });
    let archived = invoke(&window, "archive_todo", id.clone()).expect("archived");
    let listed = invoke(
        &window,
        "list_archived_todos",
        json!({ "request": { "limit": 10, "offset": 0 } }),
    )
    .expect("listed");
    assert_eq!(
        listed,
        json!({ "items": [archived["todo"]], "next_offset": null })
    );
    let deleted = invoke(&window, "delete_todo", id.clone()).expect("deleted");
    assert_eq!(deleted["todo"]["lifecycle"]["kind"], "deleted");
    assert_eq!(
        deleted["todo"]["lifecycle"]["previous"],
        archived["todo"]["lifecycle"]
    );
    assert_eq!(invoke(&window, "get_todo", id).expect("found"), deleted);
}

#[test]
fn an_assignee_round_trips_through_ipc() {
    let window = window();
    let create = |name| {
        let created = invoke(
            &window,
            "create_assignee",
            json!({ "request": { "name": name } }),
        );
        created.expect("created")["assignee"].clone()
    };
    let list = |offset| {
        invoke(
            &window,
            "list_active_assignees",
            json!({ "request": { "limit": 1, "offset": offset } }),
        )
        .expect("listed")
    };

    let ada = create("Ada");
    let grace = create("Grace");
    assert_eq!(ada["name"], "Ada");
    assert_eq!(ada["lifecycle"], json!({ "kind": "active" }));

    assert_eq!(list(0), json!({ "items": [ada], "next_offset": 1 }));
    assert_eq!(list(1), json!({ "items": [grace], "next_offset": null }));

    let deleted = invoke(
        &window,
        "delete_assignee",
        json!({ "request": { "id": ada["id"], "assigned_todo_policy": "reject" } }),
    )
    .expect("deleted");
    assert_eq!(deleted["assignee"]["lifecycle"]["kind"], "deleted");
}

#[test]
fn a_failed_command_rejects_with_a_command_error() {
    let window = window();

    let invalid = invoke(
        &window,
        "create_todo",
        json!({ "request": { "title": " ", "description": null, "assignee_id": null, "due_at": null } }),
    )
    .expect_err("invalid");
    assert_eq!(
        invalid,
        json!({
            "kind": "invalid_input",
            "code": "invalid_input",
            "message": "The input is invalid.",
            "violations": [{
                "field": "title",
                "code": "blank",
                "message": "Title must contain a non-whitespace character.",
            }],
        })
    );

    let too_many = invoke(
        &window,
        "list_archived_todos",
        json!({ "request": { "limit": 101, "offset": 0 } }),
    )
    .expect_err("too many");
    assert_eq!(too_many["violations"][0]["field"], "limit");

    let missing = invoke(
        &window,
        "get_todo",
        json!({ "request": { "id": "00000000-0000-0000-0000-000000000001" } }),
    )
    .expect_err("missing");
    assert_eq!(missing["kind"], "not_found");
    assert_eq!(missing["code"], "todo_not_found");
}

#[test]
fn a_malformed_request_rejects_with_a_message() {
    let window = window();

    for (cmd, args) in [
        ("get_todo", json!({ "request": { "id": "not-a-uuid" } })),
        (
            "get_todo",
            json!({ "request": { "id": "00000000-0000-0000-0000-000000000001", "extra": true } }),
        ),
        ("get_todo", json!({})),
    ] {
        let rejection = invoke(&window, cmd, args).expect_err("rejected");
        assert!(rejection.is_string(), "{cmd}: {rejection}");
    }
}
