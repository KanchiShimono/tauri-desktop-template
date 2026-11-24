use jiff::Timestamp;
use uuid::Uuid;

use crate::app::pagination::Page;
use crate::domain::assignee::Assignee;
use crate::domain::assignee::Lifecycle;
use crate::domain::assignee::LifecycleState;

#[derive(Debug, Eq, PartialEq)]
pub struct AssigneeView {
    id: Uuid,
    name: String,
    created_at: Timestamp,
    lifecycle: AssigneeLifecycleView,
}

impl AssigneeView {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }

    pub fn lifecycle(&self) -> AssigneeLifecycleView {
        self.lifecycle
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssigneeLifecycleView {
    Active,
    Deleted { deleted_at: Timestamp },
}

impl<L: LifecycleState> From<&Assignee<L>> for AssigneeView {
    fn from(assignee: &Assignee<L>) -> Self {
        let lifecycle = match assignee.lifecycle().into() {
            Lifecycle::Active(_) => AssigneeLifecycleView::Active,
            Lifecycle::Deleted(deleted) => AssigneeLifecycleView::Deleted {
                deleted_at: deleted.deleted_at(),
            },
        };

        Self {
            id: assignee.id().into(),
            name: assignee.name().as_ref().to_owned(),
            created_at: assignee.created_at(),
            lifecycle,
        }
    }
}

impl<L: LifecycleState> From<Page<Assignee<L>>> for Page<AssigneeView> {
    fn from(page: Page<Assignee<L>>) -> Self {
        page.map(|assignee| AssigneeView::from(&assignee))
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
