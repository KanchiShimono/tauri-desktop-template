use std::str::FromStr;

use jiff::Timestamp;
use macros::Id;
use thiserror::Error;
use uuid::Uuid;

use self::sealed::Sealed;
use crate::text;
use crate::text::NonEmptyText;
use crate::text::NonEmptyTextError;

mod sealed {
    pub trait Sealed {}
}

#[derive(Clone, Copy, Debug, Eq, Hash, Id, Ord, PartialEq, PartialOrd)]
pub struct AssigneeId(Uuid);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssigneeName(NonEmptyText);

impl AssigneeName {
    const MAX_CHARS: usize = 50;
}

impl AsRef<str> for AssigneeName {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl FromStr for AssigneeName {
    type Err = AssigneeNameError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let trimmed = value.trim();

        if trimmed.chars().nth(Self::MAX_CHARS).is_some() {
            return Err(AssigneeNameError::TooLong);
        }

        let name = trimmed.parse::<NonEmptyText>()?;
        if let Some(ch) = text::first_line_break_or_control_character(name.as_ref()) {
            return Err(AssigneeNameError::LineBreakOrControlCharacter(ch));
        }

        Ok(Self(name))
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum AssigneeNameError {
    #[error("Name must contain a non-whitespace character.")]
    Blank,
    #[error("Name contains a line break or control character ({0:?}).")]
    LineBreakOrControlCharacter(char),
    #[error("Name must be at most {max} characters.", max = AssigneeName::MAX_CHARS)]
    TooLong,
}

impl From<NonEmptyTextError> for AssigneeNameError {
    fn from(error: NonEmptyTextError) -> Self {
        match error {
            NonEmptyTextError::Empty => Self::Blank,
            NonEmptyTextError::ContainsNul => Self::LineBreakOrControlCharacter('\0'),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Active;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Deleted {
    deleted_at: Timestamp,
}

impl Deleted {
    pub fn deleted_at(&self) -> Timestamp {
        self.deleted_at
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Lifecycle {
    Active(Active),
    Deleted(Deleted),
}

impl From<Active> for Lifecycle {
    fn from(lifecycle: Active) -> Self {
        Self::Active(lifecycle)
    }
}

impl From<Deleted> for Lifecycle {
    fn from(lifecycle: Deleted) -> Self {
        Self::Deleted(lifecycle)
    }
}

pub trait LifecycleState: Copy + Into<Lifecycle> + Sealed {}

impl Sealed for Active {}
impl Sealed for Deleted {}
impl Sealed for Lifecycle {}

impl LifecycleState for Active {}
impl LifecycleState for Deleted {}
impl LifecycleState for Lifecycle {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Assignee<L: LifecycleState = Lifecycle> {
    id: AssigneeId,
    name: AssigneeName,
    created_at: Timestamp,
    lifecycle: L,
}

impl<L: LifecycleState> Assignee<L> {
    pub fn id(&self) -> AssigneeId {
        self.id
    }

    pub fn name(&self) -> &AssigneeName {
        &self.name
    }

    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }

    pub fn lifecycle(&self) -> L {
        self.lifecycle
    }

    #[must_use]
    pub fn into_any(self) -> Assignee {
        self.map_lifecycle(Into::into)
    }

    fn map_lifecycle<L2: LifecycleState>(self, f: impl FnOnce(L) -> L2) -> Assignee<L2> {
        Assignee {
            id: self.id,
            name: self.name,
            created_at: self.created_at,
            lifecycle: f(self.lifecycle),
        }
    }
}

impl Assignee<Active> {
    pub fn new(id: AssigneeId, name: AssigneeName, created_at: Timestamp) -> Self {
        Self {
            id,
            name,
            created_at,
            lifecycle: Active,
        }
    }

    #[must_use]
    pub fn rename(self, name: AssigneeName) -> Self {
        Self { name, ..self }
    }

    #[must_use]
    pub fn delete(self, deleted_at: Timestamp) -> Assignee<Deleted> {
        self.map_lifecycle(|Active| Deleted { deleted_at })
    }
}

impl Assignee<Deleted> {
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "No use case restores deleted items yet.")
    )]
    #[must_use]
    fn restore(self) -> Assignee<Active> {
        self.map_lifecycle(|Deleted { .. }| Active)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssigneeByLifecycle {
    Active(Assignee<Active>),
    Deleted(Assignee<Deleted>),
}

impl From<Assignee> for AssigneeByLifecycle {
    fn from(assignee: Assignee) -> Self {
        match assignee.lifecycle {
            Lifecycle::Active(active) => Self::Active(assignee.map_lifecycle(|_| active)),
            Lifecycle::Deleted(deleted) => Self::Deleted(assignee.map_lifecycle(|_| deleted)),
        }
    }
}

#[cfg(test)]
#[path = "assignee_tests.rs"]
mod tests;
