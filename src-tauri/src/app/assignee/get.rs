use thiserror::Error;
use uuid::Uuid;

use super::view::AssigneeView;
use crate::app::App;
use crate::app::error::AppError;
use crate::app::error::ErrorInfo;
use crate::app::error::PublicReason;
use crate::app::port::PortError;
use crate::app::port::assignee::Query;
use crate::domain::assignee::AssigneeId;

#[derive(Debug, Error)]
pub(crate) enum GetAssigneeError {
    #[error("No assignee matches {0}.")]
    NotFound(AssigneeId),
    #[error(transparent)]
    Port(#[from] PortError),
}

impl AppError for GetAssigneeError {
    fn describe(&self) -> ErrorInfo {
        match self {
            Self::NotFound(_) => ErrorInfo::NotFound(PublicReason {
                code: "assignee_not_found",
                message: self.to_string(),
            }),
            Self::Port(error) => error.describe(),
        }
    }
}

impl<S, C, I> App<S, C, I>
where
    S: Query,
{
    pub(crate) async fn get_assignee(&self, id: Uuid) -> Result<AssigneeView, GetAssigneeError> {
        let id = AssigneeId::from(id);
        let assignee = self
            .store
            .find(id)
            .await?
            .ok_or(GetAssigneeError::NotFound(id))?;
        Ok(AssigneeView::from(&assignee))
    }
}

#[cfg(test)]
#[path = "get_tests.rs"]
mod tests;
