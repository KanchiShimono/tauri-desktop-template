use domain::todo::Active;
use domain::todo::Lifecycle;
use domain::todo::LifecycleState;
use domain::todo::Status;
use domain::todo::StatusState;
use domain::todo::Todo;
use domain::todo::Undeleted;
use jiff::Timestamp;
use uuid::Uuid;

use crate::pagination::Page;

#[derive(Debug, Eq, PartialEq)]
pub struct TodoView {
    id: Uuid,
    title: String,
    description: Option<String>,
    assignee_id: Option<Uuid>,
    due_at: Option<Timestamp>,
    created_at: Timestamp,
    status: TodoStatusView,
    lifecycle: TodoLifecycleView,
}

impl TodoView {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn assignee_id(&self) -> Option<Uuid> {
        self.assignee_id
    }

    pub fn due_at(&self) -> Option<Timestamp> {
        self.due_at
    }

    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }

    pub fn status(&self) -> TodoStatusView {
        self.status
    }

    pub fn lifecycle(&self) -> TodoLifecycleView {
        self.lifecycle
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TodoStatusView {
    NotStarted,
    InProgress {
        started_at: Timestamp,
    },
    Done {
        started_at: Timestamp,
        completed_at: Timestamp,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TodoLifecycleView {
    Active,
    Archived {
        archived_at: Timestamp,
    },
    Deleted {
        deleted_at: Timestamp,
        previous: PreviousTodoLifecycleView,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviousTodoLifecycleView {
    Active,
    Archived { archived_at: Timestamp },
}

impl<S: StatusState> From<S> for TodoStatusView {
    fn from(status: S) -> Self {
        match status.into() {
            Status::NotStarted(_) => Self::NotStarted,
            Status::InProgress(progress) => Self::InProgress {
                started_at: progress.started_at().into(),
            },
            Status::Done(done) => Self::Done {
                started_at: done.started_at().into(),
                completed_at: done.completed_at().into(),
            },
        }
    }
}

impl<L: LifecycleState> From<L> for TodoLifecycleView {
    fn from(lifecycle: L) -> Self {
        match lifecycle.into() {
            Lifecycle::Active(_) => Self::Active,
            Lifecycle::Archived(archived) => Self::Archived {
                archived_at: archived.archived_at(),
            },
            Lifecycle::Deleted(deleted) => Self::Deleted {
                deleted_at: deleted.deleted_at(),
                previous: match deleted.previous() {
                    Undeleted::Active(_) => PreviousTodoLifecycleView::Active,
                    Undeleted::Archived(archived) => PreviousTodoLifecycleView::Archived {
                        archived_at: archived.archived_at(),
                    },
                },
            },
        }
    }
}

impl<L: LifecycleState, S: StatusState> From<&Todo<L, S>> for TodoView {
    fn from(todo: &Todo<L, S>) -> Self {
        Self {
            id: todo.id().into(),
            title: todo.title().as_ref().to_owned(),
            description: todo
                .description()
                .map(|description| description.as_ref().to_owned()),
            assignee_id: todo.assignee_id().map(Uuid::from),
            due_at: todo.due_at().map(Timestamp::from),
            created_at: todo.created_at(),
            status: TodoStatusView::from(todo.status()),
            lifecycle: TodoLifecycleView::from(todo.lifecycle()),
        }
    }
}

impl<L: LifecycleState, S: StatusState> From<Page<Todo<L, S>>> for Page<TodoView> {
    fn from(page: Page<Todo<L, S>>) -> Self {
        page.map(|todo| TodoView::from(&todo))
    }
}

impl From<Vec<Todo<Active, Status>>> for Page<TodoView> {
    fn from(todos: Vec<Todo<Active, Status>>) -> Self {
        Page::complete(todos).into()
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
