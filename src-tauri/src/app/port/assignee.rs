use super::PortError;
use super::UnitOfWork;
use crate::app::pagination::ListQuery;
use crate::app::pagination::Page;
use crate::domain::assignee::Active;
use crate::domain::assignee::Assignee;
use crate::domain::assignee::AssigneeId;

pub(crate) trait Query: Send + Sync {
    fn find(
        &self,
        id: AssigneeId,
    ) -> impl Future<Output = Result<Option<Assignee>, PortError>> + Send;

    /// One page of active assignees in a deterministic order with a unique tie-breaker.
    fn list_active(
        &self,
        query: ListQuery,
    ) -> impl Future<Output = Result<Page<Assignee<Active>>, PortError>> + Send;
}

pub(crate) trait Repository: UnitOfWork {
    fn load_active(
        &mut self,
        id: AssigneeId,
    ) -> impl Future<Output = Result<Option<Assignee<Active>>, PortError>> + Send;

    fn save(
        &mut self,
        assignee: &Assignee,
    ) -> impl Future<Output = Result<Assignee, PortError>> + Send;
}
