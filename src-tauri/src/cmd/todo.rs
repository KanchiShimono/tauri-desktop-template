use app::pagination::Page;
use app::pagination::UnvalidatedListQuery;
use app::presence::Presence;
use app::todo::archive::UnvalidatedArchiveTodoInput;
use app::todo::change_status::TodoStatusInput;
use app::todo::change_status::UnvalidatedChangeTodoStatusInput;
use app::todo::create::UnvalidatedCreateTodoInput;
use app::todo::delete::UnvalidatedDeleteTodoInput;
use app::todo::edit::UnvalidatedEditTodoInput;
use app::todo::unarchive::UnvalidatedUnarchiveTodoInput;
use app::todo::view::PreviousTodoLifecycleView;
use app::todo::view::TodoLifecycleView;
use app::todo::view::TodoStatusView;
use app::todo::view::TodoView;
use serde::Deserialize;
use serde::Serialize;
use specta::Type;
use tauri::State;
use uuid::Uuid;

use super::error;
use super::error::CommandError;
use super::omissible::Omissible;
use super::pagination::NextOffsetOutOfRange;
use super::timestamp::Timestamp;
use crate::App;

#[derive(Debug, Serialize, Type)]
pub(crate) struct Todo {
    id: Uuid,
    title: String,
    description: Option<String>,
    assignee_id: Option<Uuid>,
    due_at: Option<Timestamp>,
    created_at: Timestamp,
    status: TodoStatus,
    lifecycle: TodoLifecycle,
}

impl From<&TodoView> for Todo {
    fn from(view: &TodoView) -> Self {
        Self {
            id: view.id(),
            title: view.title().to_owned(),
            description: view.description().map(str::to_owned),
            assignee_id: view.assignee_id(),
            due_at: view.due_at().map(Into::into),
            created_at: view.created_at().into(),
            status: view.status().into(),
            lifecycle: view.lifecycle().into(),
        }
    }
}

#[derive(Debug, Serialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum TodoStatus {
    NotStarted,
    InProgress {
        started_at: Timestamp,
    },
    Done {
        started_at: Timestamp,
        completed_at: Timestamp,
    },
}

impl From<TodoStatusView> for TodoStatus {
    fn from(view: TodoStatusView) -> Self {
        match view {
            TodoStatusView::NotStarted => Self::NotStarted,
            TodoStatusView::InProgress { started_at } => Self::InProgress {
                started_at: started_at.into(),
            },
            TodoStatusView::Done {
                started_at,
                completed_at,
            } => Self::Done {
                started_at: started_at.into(),
                completed_at: completed_at.into(),
            },
        }
    }
}

#[derive(Debug, Serialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum TodoLifecycle {
    Active,
    Archived {
        archived_at: Timestamp,
    },
    Deleted {
        deleted_at: Timestamp,
        previous: PreviousTodoLifecycle,
    },
}

impl From<TodoLifecycleView> for TodoLifecycle {
    fn from(view: TodoLifecycleView) -> Self {
        match view {
            TodoLifecycleView::Active => Self::Active,
            TodoLifecycleView::Archived { archived_at } => Self::Archived {
                archived_at: archived_at.into(),
            },
            TodoLifecycleView::Deleted {
                deleted_at,
                previous,
            } => Self::Deleted {
                deleted_at: deleted_at.into(),
                previous: previous.into(),
            },
        }
    }
}

#[derive(Debug, Serialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum PreviousTodoLifecycle {
    Active,
    Archived { archived_at: Timestamp },
}

impl From<PreviousTodoLifecycleView> for PreviousTodoLifecycle {
    fn from(view: PreviousTodoLifecycleView) -> Self {
        match view {
            PreviousTodoLifecycleView::Active => Self::Active,
            PreviousTodoLifecycleView::Archived { archived_at } => Self::Archived {
                archived_at: archived_at.into(),
            },
        }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateTodoRequest {
    title: String,
    description: Option<String>,
    assignee_id: Option<Uuid>,
    due_at: Option<Timestamp>,
}

impl From<CreateTodoRequest> for UnvalidatedCreateTodoInput {
    fn from(request: CreateTodoRequest) -> Self {
        Self {
            title: request.title,
            description: request.description,
            assignee_id: request.assignee_id,
            due_at: request.due_at.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, Type)]
pub(crate) struct CreateTodoResponse {
    todo: Todo,
}

impl From<&TodoView> for CreateTodoResponse {
    fn from(view: &TodoView) -> Self {
        Self { todo: view.into() }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct EditTodoRequest {
    id: Uuid,
    #[serde(default)]
    title: Omissible<String>,
    #[serde(default)]
    description: Omissible<Option<String>>,
    #[serde(default)]
    assignee_id: Omissible<Option<Uuid>>,
    #[serde(default)]
    due_at: Omissible<Option<Timestamp>>,
    #[serde(default)]
    started_at: Omissible<Timestamp>,
    #[serde(default)]
    completed_at: Omissible<Timestamp>,
}

impl From<EditTodoRequest> for UnvalidatedEditTodoInput {
    fn from(request: EditTodoRequest) -> Self {
        Self {
            id: request.id,
            title: request.title.into(),
            description: request.description.into(),
            assignee_id: request.assignee_id.into(),
            due_at: Presence::from(request.due_at).map(|due_at| due_at.map(Into::into)),
            started_at: Presence::from(request.started_at).map(Into::into),
            completed_at: Presence::from(request.completed_at).map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, Type)]
pub(crate) struct EditTodoResponse {
    todo: Todo,
}

impl From<&TodoView> for EditTodoResponse {
    fn from(view: &TodoView) -> Self {
        Self { todo: view.into() }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TodoStatusKind {
    NotStarted,
    InProgress,
    Done,
}

impl From<TodoStatusKind> for TodoStatusInput {
    fn from(status: TodoStatusKind) -> Self {
        match status {
            TodoStatusKind::NotStarted => Self::NotStarted,
            TodoStatusKind::InProgress => Self::InProgress,
            TodoStatusKind::Done => Self::Done,
        }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChangeTodoStatusRequest {
    id: Uuid,
    status: TodoStatusKind,
}

impl From<ChangeTodoStatusRequest> for UnvalidatedChangeTodoStatusInput {
    fn from(request: ChangeTodoStatusRequest) -> Self {
        Self {
            id: request.id,
            status: request.status.into(),
        }
    }
}

#[derive(Debug, Serialize, Type)]
pub(crate) struct ChangeTodoStatusResponse {
    todo: Todo,
}

impl From<&TodoView> for ChangeTodoStatusResponse {
    fn from(view: &TodoView) -> Self {
        Self { todo: view.into() }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArchiveTodoRequest {
    id: Uuid,
}

impl From<ArchiveTodoRequest> for UnvalidatedArchiveTodoInput {
    fn from(request: ArchiveTodoRequest) -> Self {
        Self { id: request.id }
    }
}

#[derive(Debug, Serialize, Type)]
pub(crate) struct ArchiveTodoResponse {
    todo: Todo,
}

impl From<&TodoView> for ArchiveTodoResponse {
    fn from(view: &TodoView) -> Self {
        Self { todo: view.into() }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct UnarchiveTodoRequest {
    id: Uuid,
}

impl From<UnarchiveTodoRequest> for UnvalidatedUnarchiveTodoInput {
    fn from(request: UnarchiveTodoRequest) -> Self {
        Self { id: request.id }
    }
}

#[derive(Debug, Serialize, Type)]
pub(crate) struct UnarchiveTodoResponse {
    todo: Todo,
}

impl From<&TodoView> for UnarchiveTodoResponse {
    fn from(view: &TodoView) -> Self {
        Self { todo: view.into() }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeleteTodoRequest {
    id: Uuid,
}

impl From<DeleteTodoRequest> for UnvalidatedDeleteTodoInput {
    fn from(request: DeleteTodoRequest) -> Self {
        Self { id: request.id }
    }
}

#[derive(Debug, Serialize, Type)]
pub(crate) struct DeleteTodoResponse {
    todo: Todo,
}

impl From<&TodoView> for DeleteTodoResponse {
    fn from(view: &TodoView) -> Self {
        Self { todo: view.into() }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetTodoRequest {
    id: Uuid,
}

#[derive(Debug, Serialize, Type)]
pub(crate) struct GetTodoResponse {
    todo: Todo,
}

impl From<&TodoView> for GetTodoResponse {
    fn from(view: &TodoView) -> Self {
        Self { todo: view.into() }
    }
}

#[derive(Debug, Serialize, Type)]
pub(crate) struct ListActiveTodosResponse {
    items: Vec<Todo>,
    next_offset: Option<u32>,
}

impl TryFrom<&Page<TodoView>> for ListActiveTodosResponse {
    type Error = NextOffsetOutOfRange;

    fn try_from(page: &Page<TodoView>) -> Result<Self, Self::Error> {
        Ok(Self {
            items: page.items().iter().map(Into::into).collect(),
            next_offset: page.next_offset().map(TryInto::try_into).transpose()?,
        })
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListArchivedTodosRequest {
    limit: u32,
    offset: u32,
}

impl From<ListArchivedTodosRequest> for UnvalidatedListQuery {
    fn from(request: ListArchivedTodosRequest) -> Self {
        Self {
            limit: request.limit.try_into().unwrap_or(usize::MAX),
            offset: request.offset.try_into().unwrap_or(usize::MAX),
        }
    }
}

#[derive(Debug, Serialize, Type)]
pub(crate) struct ListArchivedTodosResponse {
    items: Vec<Todo>,
    next_offset: Option<u32>,
}

impl TryFrom<&Page<TodoView>> for ListArchivedTodosResponse {
    type Error = NextOffsetOutOfRange;

    fn try_from(page: &Page<TodoView>) -> Result<Self, Self::Error> {
        Ok(Self {
            items: page.items().iter().map(Into::into).collect(),
            next_offset: page.next_offset().map(TryInto::try_into).transpose()?,
        })
    }
}

#[tauri::command]
#[specta::specta]
#[tracing::instrument(skip_all)]
pub(crate) async fn create_todo(
    app: State<'_, App>,
    request: CreateTodoRequest,
) -> Result<CreateTodoResponse, CommandError> {
    let view = app
        .create_todo(request.into())
        .await
        .map_err(error::report)?;
    Ok((&view).into())
}

#[tauri::command]
#[specta::specta]
#[tracing::instrument(skip_all)]
pub(crate) async fn edit_todo(
    app: State<'_, App>,
    request: EditTodoRequest,
) -> Result<EditTodoResponse, CommandError> {
    let view = app.edit_todo(request.into()).await.map_err(error::report)?;
    Ok((&view).into())
}

#[tauri::command]
#[specta::specta]
#[tracing::instrument(skip_all)]
pub(crate) async fn change_todo_status(
    app: State<'_, App>,
    request: ChangeTodoStatusRequest,
) -> Result<ChangeTodoStatusResponse, CommandError> {
    let view = app
        .change_todo_status(request.into())
        .await
        .map_err(error::report)?;
    Ok((&view).into())
}

#[tauri::command]
#[specta::specta]
#[tracing::instrument(skip_all)]
pub(crate) async fn archive_todo(
    app: State<'_, App>,
    request: ArchiveTodoRequest,
) -> Result<ArchiveTodoResponse, CommandError> {
    let view = app
        .archive_todo(request.into())
        .await
        .map_err(error::report)?;
    Ok((&view).into())
}

#[tauri::command]
#[specta::specta]
#[tracing::instrument(skip_all)]
pub(crate) async fn unarchive_todo(
    app: State<'_, App>,
    request: UnarchiveTodoRequest,
) -> Result<UnarchiveTodoResponse, CommandError> {
    let view = app
        .unarchive_todo(request.into())
        .await
        .map_err(error::report)?;
    Ok((&view).into())
}

#[tauri::command]
#[specta::specta]
#[tracing::instrument(skip_all)]
pub(crate) async fn delete_todo(
    app: State<'_, App>,
    request: DeleteTodoRequest,
) -> Result<DeleteTodoResponse, CommandError> {
    let view = app
        .delete_todo(request.into())
        .await
        .map_err(error::report)?;
    Ok((&view).into())
}

#[tauri::command]
#[specta::specta]
#[tracing::instrument(skip_all)]
pub(crate) async fn get_todo(
    app: State<'_, App>,
    request: GetTodoRequest,
) -> Result<GetTodoResponse, CommandError> {
    let view = app.get_todo(request.id).await.map_err(error::report)?;
    Ok((&view).into())
}

#[tauri::command]
#[specta::specta]
#[tracing::instrument(skip_all)]
pub(crate) async fn list_active_todos(
    app: State<'_, App>,
) -> Result<ListActiveTodosResponse, CommandError> {
    let page = app.list_active_todos().await.map_err(error::report)?;
    (&page).try_into().map_err(error::report)
}

#[tauri::command]
#[specta::specta]
#[tracing::instrument(skip_all)]
pub(crate) async fn list_archived_todos(
    app: State<'_, App>,
    request: ListArchivedTodosRequest,
) -> Result<ListArchivedTodosResponse, CommandError> {
    let page = app
        .list_archived_todos(request.into())
        .await
        .map_err(error::report)?;
    (&page).try_into().map_err(error::report)
}

#[cfg(test)]
#[path = "todo_tests.rs"]
mod tests;
