#![cfg_attr(
    mobile,
    expect(
        unused_must_use,
        reason = "`tauri::mobile_entry_point` discards the result of `run`."
    )
)]

pub mod app;
mod cmd;
mod domain;
mod sqlite;
mod text;

use std::error::Error;
use std::io;

use tauri::Builder;
use tauri::Manager;
use tauri::Wry;
use tauri::async_runtime;
use tracing::Level;
use tracing_subscriber::fmt::writer::MakeWriterExt;

use crate::app::SystemClock;
use crate::app::UuidV7Generator;
use crate::sqlite::Store;

pub(crate) type App = app::App<Store, SystemClock, UuidV7Generator>;

const DATABASE_FILE: &str = "kanban.sqlite3";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(io::stderr.with_max_level(Level::WARN).or_else(io::stdout))
        .init();

    let commands = cmd::builder::<Wry>();

    Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(commands.invoke_handler())
        .setup(|app| {
            let path = app.path().app_data_dir()?.join(DATABASE_FILE);
            let store = async_runtime::block_on(Store::open(&path)).inspect_err(|error| {
                tracing::error!(?path, error = error as &dyn Error, "startup failed");
            })?;
            app.manage(App::new(store, SystemClock, UuidV7Generator));
            Ok(())
        })
        .run(tauri::generate_context!())
}
