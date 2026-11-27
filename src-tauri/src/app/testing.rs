//! In-memory ports for use-case tests.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering as AtomicOrdering;

use domain::assignee;
use domain::assignee::Assignee;
use domain::assignee::AssigneeByLifecycle;
use domain::assignee::AssigneeId;
use domain::todo::Active;
use domain::todo::Archived;
use domain::todo::Status;
use domain::todo::Todo;
use domain::todo::TodoByLifecycle;
use domain::todo::TodoId;
use domain::todo::Undeleted;
use jiff::Timestamp;
use uuid::Uuid;

use super::App;
use super::pagination::ListQuery;
use super::pagination::Page;
use super::port;
use super::port::Clock;
use super::port::IdGenerator;
use super::port::PortError;
use super::port::UnitOfWork;
use super::port::UnitOfWorkFactory;

pub(crate) type TestApp = App<MemoryStore, ManualClock, SequentialIds>;

pub(crate) fn app() -> TestApp {
    App::new(
        MemoryStore::default(),
        ManualClock::default(),
        SequentialIds::default(),
    )
}

impl TestApp {
    pub(crate) fn set_time(&self, now: Timestamp) {
        *self.clock.0.lock().expect("clock lock") = now;
    }
}

#[derive(Debug)]
pub(crate) struct ManualClock(Mutex<Timestamp>);

impl Default for ManualClock {
    fn default() -> Self {
        Self(Mutex::new(Timestamp::UNIX_EPOCH))
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Timestamp {
        *self.0.lock().expect("clock lock")
    }
}

/// Generates 1, 2, 3, ... so tests can name the identifiers they expect.
#[derive(Debug, Default)]
pub(crate) struct SequentialIds(AtomicU64);

impl<Id: From<Uuid>> IdGenerator<Id> for SequentialIds {
    fn generate(&self) -> Id {
        let next = self.0.fetch_add(1, AtomicOrdering::Relaxed) + 1;
        Id::from(Uuid::from_u128(next.into()))
    }
}

#[derive(Clone, Debug, Default)]
struct State {
    assignees: BTreeMap<AssigneeId, Assignee>,
    todos: BTreeMap<TodoId, Todo>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct MemoryStore(Arc<Mutex<State>>);

impl MemoryStore {
    fn snapshot(&self) -> State {
        self.0.lock().expect("store lock").clone()
    }
}

/// Works on a private copy that replaces the shared state only on commit.
#[derive(Debug)]
pub(crate) struct MemoryUnitOfWork {
    store: MemoryStore,
    state: State,
}

impl UnitOfWorkFactory for MemoryStore {
    type UnitOfWork = MemoryUnitOfWork;

    async fn begin(&self) -> Result<Self::UnitOfWork, PortError> {
        Ok(MemoryUnitOfWork {
            store: self.clone(),
            state: self.snapshot(),
        })
    }
}

impl UnitOfWork for MemoryUnitOfWork {
    async fn commit(self) -> Result<(), PortError> {
        *self.store.0.lock().expect("store lock") = self.state;
        Ok(())
    }

    async fn rollback(self) -> Result<(), PortError> {
        Ok(())
    }
}

fn page<T>(query: ListQuery, rows: impl Iterator<Item = T>) -> Result<Page<T>, PortError> {
    let rows = rows.skip(query.offset()).take(query.limit() + 1).collect();
    Ok(Page::from_lookahead(query, rows)?)
}

impl port::assignee::Query for MemoryStore {
    async fn find(&self, id: AssigneeId) -> Result<Option<Assignee>, PortError> {
        Ok(self.snapshot().assignees.remove(&id))
    }

    async fn list_active(
        &self,
        query: ListQuery,
    ) -> Result<Page<Assignee<assignee::Active>>, PortError> {
        let rows = self.snapshot().assignees.into_values();
        page(
            query,
            rows.filter_map(|assignee| match AssigneeByLifecycle::from(assignee) {
                AssigneeByLifecycle::Active(assignee) => Some(assignee),
                AssigneeByLifecycle::Deleted(_) => None,
            }),
        )
    }
}

impl port::assignee::Repository for MemoryUnitOfWork {
    async fn load_active(
        &mut self,
        id: AssigneeId,
    ) -> Result<Option<Assignee<assignee::Active>>, PortError> {
        let assignee = self.state.assignees.get(&id).cloned();
        Ok(
            assignee.and_then(|assignee| match AssigneeByLifecycle::from(assignee) {
                AssigneeByLifecycle::Active(assignee) => Some(assignee),
                AssigneeByLifecycle::Deleted(_) => None,
            }),
        )
    }

    async fn save(&mut self, assignee: &Assignee) -> Result<Assignee, PortError> {
        self.state.assignees.insert(assignee.id(), assignee.clone());
        Ok(assignee.clone())
    }
}

fn undeleted(todo: Todo) -> Option<Todo<Undeleted, Status>> {
    match TodoByLifecycle::from(todo) {
        TodoByLifecycle::Active(todo) => Some(todo.into()),
        TodoByLifecycle::Archived(todo) => Some(todo.into()),
        TodoByLifecycle::Deleted(_) => None,
    }
}

impl port::todo::Query for MemoryStore {
    async fn find(&self, id: TodoId) -> Result<Option<Todo>, PortError> {
        Ok(self.snapshot().todos.remove(&id))
    }

    async fn list_active(&self) -> Result<Vec<Todo<Active, Status>>, PortError> {
        let rows = self.snapshot().todos.into_values();
        Ok(rows
            .filter_map(|todo| match TodoByLifecycle::from(todo) {
                TodoByLifecycle::Active(todo) => Some(todo),
                TodoByLifecycle::Archived(_) | TodoByLifecycle::Deleted(_) => None,
            })
            .collect())
    }

    async fn list_archived(
        &self,
        query: ListQuery,
    ) -> Result<Page<Todo<Archived, Status>>, PortError> {
        let rows = self.snapshot().todos.into_values();
        page(
            query,
            rows.filter_map(|todo| match TodoByLifecycle::from(todo) {
                TodoByLifecycle::Archived(todo) => Some(todo),
                TodoByLifecycle::Active(_) | TodoByLifecycle::Deleted(_) => None,
            }),
        )
    }
}

impl port::todo::Repository for MemoryUnitOfWork {
    async fn load_undeleted(
        &mut self,
        id: TodoId,
    ) -> Result<Option<Todo<Undeleted, Status>>, PortError> {
        Ok(self.state.todos.get(&id).cloned().and_then(undeleted))
    }

    async fn load_all_for(&mut self, assignee_id: AssigneeId) -> Result<Vec<Todo>, PortError> {
        let todos = self.state.todos.values();
        Ok(todos
            .filter(|todo| todo.assignee_id() == Some(assignee_id))
            .cloned()
            .collect())
    }

    async fn save(&mut self, todo: &Todo) -> Result<Todo, PortError> {
        self.state.todos.insert(todo.id(), todo.clone());
        Ok(todo.clone())
    }

    async fn has_undeleted_for(&mut self, assignee_id: AssigneeId) -> Result<bool, PortError> {
        Ok(self
            .state
            .todos
            .values()
            .filter(|todo| todo.assignee_id() == Some(assignee_id))
            .any(|todo| undeleted(todo.clone()).is_some()))
    }
}
