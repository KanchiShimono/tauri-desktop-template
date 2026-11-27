use app::assignee::create::UnvalidatedCreateAssigneeInput;
use app::assignee::delete;
use app::assignee::delete::UnvalidatedDeleteAssigneeInput;
use app::assignee::rename::UnvalidatedRenameAssigneeInput;
use app::assignee::view::AssigneeLifecycleView;
use app::assignee::view::AssigneeView;
use app::pagination::Page;
use app::pagination::UnvalidatedListQuery;
use serde::Deserialize;
use serde::Serialize;
use specta::Type;
use tauri::State;
use uuid::Uuid;

use super::error;
use super::error::CommandError;
use super::pagination::NextOffsetOutOfRange;
use super::timestamp::Timestamp;
use crate::App;

#[derive(Debug, Serialize, Type)]
pub(crate) struct Assignee {
    id: Uuid,
    name: String,
    created_at: Timestamp,
    lifecycle: AssigneeLifecycle,
}

impl From<&AssigneeView> for Assignee {
    fn from(view: &AssigneeView) -> Self {
        Self {
            id: view.id(),
            name: view.name().to_owned(),
            created_at: view.created_at().into(),
            lifecycle: view.lifecycle().into(),
        }
    }
}

#[derive(Debug, Serialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum AssigneeLifecycle {
    Active,
    Deleted { deleted_at: Timestamp },
}

impl From<AssigneeLifecycleView> for AssigneeLifecycle {
    fn from(view: AssigneeLifecycleView) -> Self {
        match view {
            AssigneeLifecycleView::Active => Self::Active,
            AssigneeLifecycleView::Deleted { deleted_at } => Self::Deleted {
                deleted_at: deleted_at.into(),
            },
        }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateAssigneeRequest {
    name: String,
}

impl From<CreateAssigneeRequest> for UnvalidatedCreateAssigneeInput {
    fn from(request: CreateAssigneeRequest) -> Self {
        Self { name: request.name }
    }
}

#[derive(Debug, Serialize, Type)]
pub(crate) struct CreateAssigneeResponse {
    assignee: Assignee,
}

impl From<&AssigneeView> for CreateAssigneeResponse {
    fn from(view: &AssigneeView) -> Self {
        Self {
            assignee: view.into(),
        }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct RenameAssigneeRequest {
    id: Uuid,
    name: String,
}

impl From<RenameAssigneeRequest> for UnvalidatedRenameAssigneeInput {
    fn from(request: RenameAssigneeRequest) -> Self {
        Self {
            id: request.id,
            name: request.name,
        }
    }
}

#[derive(Debug, Serialize, Type)]
pub(crate) struct RenameAssigneeResponse {
    assignee: Assignee,
}

impl From<&AssigneeView> for RenameAssigneeResponse {
    fn from(view: &AssigneeView) -> Self {
        Self {
            assignee: view.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AssignedTodoPolicy {
    Reject,
    Unassign,
}

impl From<AssignedTodoPolicy> for delete::AssignedTodoPolicy {
    fn from(policy: AssignedTodoPolicy) -> Self {
        match policy {
            AssignedTodoPolicy::Reject => Self::Reject,
            AssignedTodoPolicy::Unassign => Self::Unassign,
        }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeleteAssigneeRequest {
    id: Uuid,
    assigned_todo_policy: AssignedTodoPolicy,
}

impl From<DeleteAssigneeRequest> for UnvalidatedDeleteAssigneeInput {
    fn from(request: DeleteAssigneeRequest) -> Self {
        Self {
            id: request.id,
            assigned_todo_policy: request.assigned_todo_policy.into(),
        }
    }
}

#[derive(Debug, Serialize, Type)]
pub(crate) struct DeleteAssigneeResponse {
    assignee: Assignee,
}

impl From<&AssigneeView> for DeleteAssigneeResponse {
    fn from(view: &AssigneeView) -> Self {
        Self {
            assignee: view.into(),
        }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetAssigneeRequest {
    id: Uuid,
}

#[derive(Debug, Serialize, Type)]
pub(crate) struct GetAssigneeResponse {
    assignee: Assignee,
}

impl From<&AssigneeView> for GetAssigneeResponse {
    fn from(view: &AssigneeView) -> Self {
        Self {
            assignee: view.into(),
        }
    }
}

#[derive(Debug, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListActiveAssigneesRequest {
    limit: u32,
    offset: u32,
}

impl From<ListActiveAssigneesRequest> for UnvalidatedListQuery {
    fn from(request: ListActiveAssigneesRequest) -> Self {
        Self {
            limit: request.limit.try_into().unwrap_or(usize::MAX),
            offset: request.offset.try_into().unwrap_or(usize::MAX),
        }
    }
}

#[derive(Debug, Serialize, Type)]
pub(crate) struct ListActiveAssigneesResponse {
    items: Vec<Assignee>,
    next_offset: Option<u32>,
}

impl TryFrom<&Page<AssigneeView>> for ListActiveAssigneesResponse {
    type Error = NextOffsetOutOfRange;

    fn try_from(page: &Page<AssigneeView>) -> Result<Self, Self::Error> {
        Ok(Self {
            items: page.items().iter().map(Into::into).collect(),
            next_offset: page.next_offset().map(TryInto::try_into).transpose()?,
        })
    }
}

#[tauri::command]
#[specta::specta]
#[tracing::instrument(skip_all)]
pub(crate) async fn create_assignee(
    app: State<'_, App>,
    request: CreateAssigneeRequest,
) -> Result<CreateAssigneeResponse, CommandError> {
    let view = app
        .create_assignee(request.into())
        .await
        .map_err(error::report)?;
    Ok((&view).into())
}

#[tauri::command]
#[specta::specta]
#[tracing::instrument(skip_all)]
pub(crate) async fn rename_assignee(
    app: State<'_, App>,
    request: RenameAssigneeRequest,
) -> Result<RenameAssigneeResponse, CommandError> {
    let view = app
        .rename_assignee(request.into())
        .await
        .map_err(error::report)?;
    Ok((&view).into())
}

#[tauri::command]
#[specta::specta]
#[tracing::instrument(skip_all)]
pub(crate) async fn delete_assignee(
    app: State<'_, App>,
    request: DeleteAssigneeRequest,
) -> Result<DeleteAssigneeResponse, CommandError> {
    let view = app
        .delete_assignee(request.into())
        .await
        .map_err(error::report)?;
    Ok((&view).into())
}

#[tauri::command]
#[specta::specta]
#[tracing::instrument(skip_all)]
pub(crate) async fn get_assignee(
    app: State<'_, App>,
    request: GetAssigneeRequest,
) -> Result<GetAssigneeResponse, CommandError> {
    let view = app.get_assignee(request.id).await.map_err(error::report)?;
    Ok((&view).into())
}

#[tauri::command]
#[specta::specta]
#[tracing::instrument(skip_all)]
pub(crate) async fn list_active_assignees(
    app: State<'_, App>,
    request: ListActiveAssigneesRequest,
) -> Result<ListActiveAssigneesResponse, CommandError> {
    let page = app
        .list_active_assignees(request.into())
        .await
        .map_err(error::report)?;
    (&page).try_into().map_err(error::report)
}

#[cfg(test)]
#[path = "assignee_tests.rs"]
mod tests;
