mod assignee;
mod error;
mod omissible;
mod pagination;
mod timestamp;
mod todo;

use tauri::Runtime;
use tauri_specta::Builder;
use tauri_specta::ErrorHandlingMode;

use self::error::CommandError;
use crate::app::ListQuery;

pub(crate) fn builder<R: Runtime>() -> Builder<R> {
    Builder::new()
        .commands(tauri_specta::collect_commands![
            assignee::create_assignee,
            assignee::delete_assignee,
            assignee::get_assignee,
            assignee::list_active_assignees,
            assignee::rename_assignee,
            todo::archive_todo,
            todo::change_todo_status,
            todo::create_todo,
            todo::delete_todo,
            todo::edit_todo,
            todo::get_todo,
            todo::list_active_todos,
            todo::list_archived_todos,
            todo::unarchive_todo,
        ])
        .typ::<CommandError>()
        .constant("MAX_LIST_LIMIT", ListQuery::MAX_LIMIT)
        .error_handling(ErrorHandlingMode::Throw)
        .disable_serde_phases()
}

#[cfg(test)]
#[path = "cmd_tests.rs"]
mod tests;
