use std::env;
use std::fs;
use std::process;

use specta_typescript::Typescript;
use tauri::Wry;

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
