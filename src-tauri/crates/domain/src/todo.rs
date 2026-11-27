use std::str::FromStr;

use jiff::Timestamp;
use macros::From;
use macros::Id;
use macros::Into;
use thiserror::Error;
use uuid::Uuid;

use self::sealed::SealedLifecycle;
use self::sealed::SealedStatus;
use self::sealed::SealedUndeleted;
use crate::assignee;
use crate::assignee::Assignee;
use crate::assignee::AssigneeId;
use crate::text;
use crate::text::NonEmptyText;
use crate::text::NonEmptyTextError;

mod sealed {
    pub trait SealedLifecycle {}
    pub trait SealedStatus {}
    pub trait SealedUndeleted {}
}

#[derive(Clone, Copy, Debug, Eq, Hash, Id, Ord, PartialEq, PartialOrd)]
pub struct TodoId(Uuid);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TodoTitle(NonEmptyText);

impl TodoTitle {
    const MAX_CHARS: usize = 100;
}

impl AsRef<str> for TodoTitle {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl FromStr for TodoTitle {
    type Err = TodoTitleError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let trimmed = value.trim();

        if trimmed.chars().nth(Self::MAX_CHARS).is_some() {
            return Err(TodoTitleError::TooLong);
        }

        let title = trimmed.parse::<NonEmptyText>()?;
        if let Some(ch) = text::first_line_break_or_control_character(title.as_ref()) {
            return Err(TodoTitleError::LineBreakOrControlCharacter(ch));
        }

        Ok(Self(title))
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum TodoTitleError {
    #[error("Title must contain a non-whitespace character.")]
    Blank,
    #[error("Title contains a line break or control character ({0:?}).")]
    LineBreakOrControlCharacter(char),
    #[error("Title must be at most {max} characters.", max = TodoTitle::MAX_CHARS)]
    TooLong,
}

impl From<NonEmptyTextError> for TodoTitleError {
    fn from(error: NonEmptyTextError) -> Self {
        match error {
            NonEmptyTextError::Empty => Self::Blank,
            NonEmptyTextError::ContainsNul => Self::LineBreakOrControlCharacter('\0'),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TodoDescription(NonEmptyText);

impl TodoDescription {
    const MAX_CHARS: usize = 2_000;

    pub fn try_from_optional(value: Option<&str>) -> Result<Option<Self>, TodoDescriptionError> {
        let Some(value) = value else {
            return Ok(None);
        };
        match value.parse::<Self>() {
            Err(TodoDescriptionError::Blank) => Ok(None),
            result => result.map(Some),
        }
    }
}

impl AsRef<str> for TodoDescription {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl FromStr for TodoDescription {
    type Err = TodoDescriptionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.trim().is_empty() {
            return Err(NonEmptyTextError::Empty.into());
        }

        let other_line_endings = ['\r', '\u{0085}', '\u{2028}', '\u{2029}'];
        let normalized = value
            .replace("\r\n", "\n")
            .replace(other_line_endings, "\n");
        if normalized.chars().nth(Self::MAX_CHARS).is_some() {
            return Err(TodoDescriptionError::TooLong);
        }

        let description = normalized.parse::<NonEmptyText>()?;
        if description
            .as_ref()
            .contains(|ch: char| ch.is_control() && !matches!(ch, '\n' | '\t'))
        {
            return Err(TodoDescriptionError::InvalidControlCharacter);
        }

        Ok(Self(description))
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum TodoDescriptionError {
    #[error("Description must contain a non-whitespace character.")]
    Blank,
    #[error("Description may only contain tabs and line breaks as control characters.")]
    InvalidControlCharacter,
    #[error("Description must be at most {max} characters.", max = TodoDescription::MAX_CHARS)]
    TooLong,
}

impl From<NonEmptyTextError> for TodoDescriptionError {
    fn from(error: NonEmptyTextError) -> Self {
        match error {
            NonEmptyTextError::Empty => Self::Blank,
            NonEmptyTextError::ContainsNul => Self::InvalidControlCharacter,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, From, Hash, Into, Ord, PartialEq, PartialOrd)]
pub struct StartedAt(Timestamp);

#[derive(Clone, Copy, Debug, Eq, From, Hash, Into, Ord, PartialEq, PartialOrd)]
pub struct CompletedAt(Timestamp);

#[derive(Clone, Copy, Debug, Eq, From, Hash, Into, Ord, PartialEq, PartialOrd)]
pub struct DueAt(Timestamp);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Active;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Archived {
    archived_at: Timestamp,
}

impl Archived {
    pub fn archived_at(&self) -> Timestamp {
        self.archived_at
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Undeleted {
    Active(Active),
    Archived(Archived),
}

impl From<Active> for Undeleted {
    fn from(lifecycle: Active) -> Self {
        Self::Active(lifecycle)
    }
}

impl From<Archived> for Undeleted {
    fn from(lifecycle: Archived) -> Self {
        Self::Archived(lifecycle)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Deleted<P: UndeletedState = Undeleted> {
    previous: P,
    deleted_at: Timestamp,
}

impl<P: UndeletedState> Deleted<P> {
    pub fn previous(&self) -> P {
        self.previous
    }

    pub fn deleted_at(&self) -> Timestamp {
        self.deleted_at
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Lifecycle {
    Active(Active),
    Archived(Archived),
    Deleted(Deleted),
}

impl From<Active> for Lifecycle {
    fn from(lifecycle: Active) -> Self {
        Self::Active(lifecycle)
    }
}

impl From<Archived> for Lifecycle {
    fn from(lifecycle: Archived) -> Self {
        Self::Archived(lifecycle)
    }
}

impl From<Undeleted> for Lifecycle {
    fn from(lifecycle: Undeleted) -> Self {
        match lifecycle {
            Undeleted::Active(active) => Self::Active(active),
            Undeleted::Archived(archived) => Self::Archived(archived),
        }
    }
}

impl<P: UndeletedState> From<Deleted<P>> for Lifecycle {
    fn from(lifecycle: Deleted<P>) -> Self {
        Self::Deleted(Deleted {
            previous: lifecycle.previous.into(),
            deleted_at: lifecycle.deleted_at,
        })
    }
}

pub trait LifecycleState: Copy + Into<Lifecycle> + SealedLifecycle {}

pub trait UndeletedState: LifecycleState + Into<Undeleted> + SealedUndeleted {}

impl SealedLifecycle for Active {}
impl SealedLifecycle for Archived {}
impl SealedLifecycle for Undeleted {}
impl<P: UndeletedState> SealedLifecycle for Deleted<P> {}
impl SealedLifecycle for Lifecycle {}

impl LifecycleState for Active {}
impl LifecycleState for Archived {}
impl LifecycleState for Undeleted {}
impl<P: UndeletedState> LifecycleState for Deleted<P> {}
impl LifecycleState for Lifecycle {}

impl SealedUndeleted for Active {}
impl SealedUndeleted for Archived {}
impl SealedUndeleted for Undeleted {}

impl UndeletedState for Active {}
impl UndeletedState for Archived {}
impl UndeletedState for Undeleted {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NotStarted;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct InProgress {
    started_at: StartedAt,
}

impl InProgress {
    pub fn started_at(&self) -> StartedAt {
        self.started_at
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Done {
    started_at: StartedAt,
    completed_at: CompletedAt,
}

impl Done {
    pub fn started_at(&self) -> StartedAt {
        self.started_at
    }

    pub fn completed_at(&self) -> CompletedAt {
        self.completed_at
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Status {
    NotStarted(NotStarted),
    InProgress(InProgress),
    Done(Done),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StatusKind {
    NotStarted,
    InProgress,
    Done,
}

impl From<Status> for StatusKind {
    fn from(status: Status) -> Self {
        match status {
            Status::NotStarted(_) => Self::NotStarted,
            Status::InProgress(_) => Self::InProgress,
            Status::Done(_) => Self::Done,
        }
    }
}

impl From<NotStarted> for Status {
    fn from(status: NotStarted) -> Self {
        Self::NotStarted(status)
    }
}

impl From<InProgress> for Status {
    fn from(status: InProgress) -> Self {
        Self::InProgress(status)
    }
}

impl From<Done> for Status {
    fn from(status: Done) -> Self {
        Self::Done(status)
    }
}

pub trait StatusState: Copy + Into<Status> + SealedStatus {}

impl SealedStatus for NotStarted {}
impl SealedStatus for InProgress {}
impl SealedStatus for Done {}
impl SealedStatus for Status {}

impl StatusState for NotStarted {}
impl StatusState for InProgress {}
impl StatusState for Done {}
impl StatusState for Status {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Todo<L: LifecycleState = Lifecycle, S: StatusState = Status> {
    id: TodoId,
    title: TodoTitle,
    description: Option<TodoDescription>,
    assignee_id: Option<AssigneeId>,
    due_at: Option<DueAt>,
    created_at: Timestamp,
    lifecycle: L,
    status: S,
}

impl<L: LifecycleState, S: StatusState> Todo<L, S> {
    pub fn id(&self) -> TodoId {
        self.id
    }

    pub fn title(&self) -> &TodoTitle {
        &self.title
    }

    pub fn description(&self) -> Option<&TodoDescription> {
        self.description.as_ref()
    }

    pub fn assignee_id(&self) -> Option<AssigneeId> {
        self.assignee_id
    }

    pub fn due_at(&self) -> Option<DueAt> {
        self.due_at
    }

    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }

    pub fn lifecycle(&self) -> L {
        self.lifecycle
    }

    pub fn status(&self) -> S {
        self.status
    }

    #[must_use]
    pub fn unassign_for_deleted_assignee(self, assignee: &Assignee<assignee::Deleted>) -> Self {
        if self.assignee_id == Some(assignee.id()) {
            Self {
                assignee_id: None,
                ..self
            }
        } else {
            self
        }
    }

    #[must_use]
    pub fn into_any(self) -> Todo {
        self.map_lifecycle(Into::into).map_status(Into::into)
    }

    fn map_lifecycle<L2: LifecycleState>(self, f: impl FnOnce(L) -> L2) -> Todo<L2, S> {
        Todo {
            id: self.id,
            title: self.title,
            description: self.description,
            assignee_id: self.assignee_id,
            due_at: self.due_at,
            created_at: self.created_at,
            lifecycle: f(self.lifecycle),
            status: self.status,
        }
    }

    fn map_status<S2: StatusState>(self, f: impl FnOnce(S) -> S2) -> Todo<L, S2> {
        Todo {
            id: self.id,
            title: self.title,
            description: self.description,
            assignee_id: self.assignee_id,
            due_at: self.due_at,
            created_at: self.created_at,
            lifecycle: self.lifecycle,
            status: f(self.status),
        }
    }
}

impl Todo<Active, NotStarted> {
    pub fn new(
        id: TodoId,
        title: TodoTitle,
        description: Option<TodoDescription>,
        assignee_id: Option<AssigneeId>,
        due_at: Option<DueAt>,
        created_at: Timestamp,
    ) -> Self {
        Self {
            id,
            title,
            description,
            assignee_id,
            due_at,
            created_at,
            lifecycle: Active,
            status: NotStarted,
        }
    }

    #[must_use]
    pub fn start(self, started_at: StartedAt) -> Todo<Active, InProgress> {
        self.map_status(|NotStarted| InProgress { started_at })
    }
}

impl Todo<Active, InProgress> {
    #[must_use]
    pub fn change_started_at(self, started_at: StartedAt) -> Self {
        self.map_status(|_| InProgress { started_at })
    }

    #[must_use]
    fn unstart(self) -> Todo<Active, NotStarted> {
        self.map_status(|_| NotStarted)
    }

    #[must_use]
    pub fn complete(self, completed_at: CompletedAt) -> Todo<Active, Done> {
        self.map_status(|InProgress { started_at }| Done {
            started_at,
            completed_at,
        })
    }
}

impl Todo<Active, Done> {
    #[must_use]
    pub fn change_started_at(self, started_at: StartedAt) -> Self {
        self.map_status(|done| Done { started_at, ..done })
    }

    #[must_use]
    pub fn change_completed_at(self, completed_at: CompletedAt) -> Self {
        self.map_status(|done| Done {
            completed_at,
            ..done
        })
    }

    #[must_use]
    fn reopen(self) -> Todo<Active, InProgress> {
        self.map_status(|done| InProgress {
            started_at: done.started_at,
        })
    }
}

impl<S: StatusState> Todo<Active, S> {
    #[must_use]
    pub fn change_status(self, target: StatusKind, at: Timestamp) -> Todo<Active, Status> {
        let current = TodoByStatus::from(self.map_status(Into::into));
        match target {
            StatusKind::NotStarted => match current {
                TodoByStatus::NotStarted(todo) => todo,
                TodoByStatus::InProgress(todo) => todo.unstart(),
                TodoByStatus::Done(todo) => todo.reopen().unstart(),
            }
            .map_status(Into::into),
            StatusKind::InProgress => match current {
                TodoByStatus::NotStarted(todo) => todo.start(at.into()),
                TodoByStatus::InProgress(todo) => todo,
                TodoByStatus::Done(todo) => todo.reopen(),
            }
            .map_status(Into::into),
            StatusKind::Done => match current {
                TodoByStatus::NotStarted(todo) => todo.start(at.into()).complete(at.into()),
                TodoByStatus::InProgress(todo) => todo.complete(at.into()),
                TodoByStatus::Done(todo) => todo,
            }
            .map_status(Into::into),
        }
    }

    #[must_use]
    pub fn rename(self, title: TodoTitle) -> Self {
        Self { title, ..self }
    }

    #[must_use]
    pub fn change_description(self, description: Option<TodoDescription>) -> Self {
        Self {
            description,
            ..self
        }
    }

    #[must_use]
    pub fn reassign(self, assignee: &Assignee<assignee::Active>) -> Self {
        Self {
            assignee_id: Some(assignee.id()),
            ..self
        }
    }

    #[must_use]
    pub fn unassign(self) -> Self {
        Self {
            assignee_id: None,
            ..self
        }
    }

    #[must_use]
    pub fn change_due_at(self, due_at: Option<DueAt>) -> Self {
        Self { due_at, ..self }
    }

    #[must_use]
    pub fn archive(self, archived_at: Timestamp) -> Todo<Archived, S> {
        self.map_lifecycle(|Active| Archived { archived_at })
    }
}

impl<S: StatusState> Todo<Archived, S> {
    #[must_use]
    pub fn unarchive(self) -> Todo<Active, S> {
        self.map_lifecycle(|Archived { .. }| Active)
    }
}

impl<P: UndeletedState, S: StatusState> Todo<P, S> {
    #[must_use]
    pub fn delete(self, deleted_at: Timestamp) -> Todo<Deleted<P>, S> {
        self.map_lifecycle(|previous| Deleted {
            previous,
            deleted_at,
        })
    }
}

impl<P: UndeletedState, S: StatusState> Todo<Deleted<P>, S> {
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "No use case restores deleted items yet.")
    )]
    #[must_use]
    fn restore(self) -> Todo<P, S> {
        self.map_lifecycle(|Deleted { previous, .. }| previous)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TodoByLifecycle<S: StatusState> {
    Active(Todo<Active, S>),
    Archived(Todo<Archived, S>),
    Deleted(Todo<Deleted, S>),
}

impl<S: StatusState> From<Todo<Lifecycle, S>> for TodoByLifecycle<S> {
    fn from(todo: Todo<Lifecycle, S>) -> Self {
        match todo.lifecycle {
            Lifecycle::Active(active) => Self::Active(todo.map_lifecycle(|_| active)),
            Lifecycle::Archived(archived) => Self::Archived(todo.map_lifecycle(|_| archived)),
            Lifecycle::Deleted(deleted) => Self::Deleted(todo.map_lifecycle(|_| deleted)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TodoByUndeleted<S: StatusState> {
    Active(Todo<Active, S>),
    Archived(Todo<Archived, S>),
}

impl<S: StatusState> From<Todo<Undeleted, S>> for TodoByUndeleted<S> {
    fn from(todo: Todo<Undeleted, S>) -> Self {
        match todo.lifecycle {
            Undeleted::Active(active) => Self::Active(todo.map_lifecycle(|_| active)),
            Undeleted::Archived(archived) => Self::Archived(todo.map_lifecycle(|_| archived)),
        }
    }
}

impl<S: StatusState> From<Todo<Active, S>> for Todo<Undeleted, S> {
    fn from(todo: Todo<Active, S>) -> Self {
        todo.map_lifecycle(Into::into)
    }
}

impl<S: StatusState> From<Todo<Archived, S>> for Todo<Undeleted, S> {
    fn from(todo: Todo<Archived, S>) -> Self {
        todo.map_lifecycle(Into::into)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TodoByStatus<L: LifecycleState> {
    NotStarted(Todo<L, NotStarted>),
    InProgress(Todo<L, InProgress>),
    Done(Todo<L, Done>),
}

impl<L: LifecycleState> From<Todo<L, Status>> for TodoByStatus<L> {
    fn from(todo: Todo<L, Status>) -> Self {
        match todo.status {
            Status::NotStarted(status) => Self::NotStarted(todo.map_status(|_| status)),
            Status::InProgress(status) => Self::InProgress(todo.map_status(|_| status)),
            Status::Done(status) => Self::Done(todo.map_status(|_| status)),
        }
    }
}

#[cfg(test)]
#[path = "todo_tests.rs"]
mod tests;
