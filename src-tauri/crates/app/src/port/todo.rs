use domain::assignee::AssigneeId;
use domain::todo::Active;
use domain::todo::Archived;
use domain::todo::Status;
use domain::todo::Todo;
use domain::todo::TodoId;
use domain::todo::Undeleted;

use crate::pagination::ListQuery;
use crate::pagination::Page;
use crate::port::PortError;
use crate::port::UnitOfWork;

pub trait Query: Send + Sync {
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

pub trait Repository: UnitOfWork {
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
