use crate::app::pagination::ListQuery;
use crate::app::pagination::Page;
use crate::app::port::PortError;
use crate::app::port::UnitOfWork;
use crate::domain::assignee::AssigneeId;
use crate::domain::todo::Active;
use crate::domain::todo::Archived;
use crate::domain::todo::Status;
use crate::domain::todo::Todo;
use crate::domain::todo::TodoId;
use crate::domain::todo::Undeleted;

pub(crate) trait Query: Send + Sync {
    fn find(&self, id: TodoId) -> impl Future<Output = Result<Option<Todo>, PortError>> + Send;

    /// All active Todos, including collections larger than the paginated query limit.
    fn list_active(
        &self,
    ) -> impl Future<Output = Result<Vec<Todo<Active, Status>>, PortError>> + Send;

    /// One page of undeleted, archived Todos in a deterministic order with a unique tie-breaker.
    fn list_archived(
        &self,
        query: ListQuery,
    ) -> impl Future<Output = Result<Page<Todo<Archived, Status>>, PortError>> + Send;
}

pub(crate) trait Repository: UnitOfWork {
    fn load_undeleted(
        &mut self,
        id: TodoId,
    ) -> impl Future<Output = Result<Option<Todo<Undeleted, Status>>, PortError>> + Send;

    /// Every Todo assigned to the assignee, in any lifecycle.
    fn load_all_for(
        &mut self,
        assignee_id: AssigneeId,
    ) -> impl Future<Output = Result<Vec<Todo>, PortError>> + Send;

    fn save(&mut self, todo: &Todo) -> impl Future<Output = Result<Todo, PortError>> + Send;

    fn has_undeleted_for(
        &mut self,
        assignee_id: AssigneeId,
    ) -> impl Future<Output = Result<bool, PortError>> + Send;
}
