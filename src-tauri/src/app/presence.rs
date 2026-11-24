use std::ops::Deref;
use std::ops::DerefMut;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Presence<T> {
    #[default]
    Absent,
    Present(T),
}

impl<T> Presence<T> {
    #[must_use]
    pub const fn is_present(&self) -> bool {
        matches!(self, Presence::Present(_))
    }

    #[must_use]
    pub const fn is_absent(&self) -> bool {
        matches!(self, Presence::Absent)
    }

    #[must_use]
    pub const fn as_ref(&self) -> Presence<&T> {
        match self {
            Presence::Absent => Presence::Absent,
            Presence::Present(value) => Presence::Present(value),
        }
    }

    #[must_use]
    pub const fn as_mut(&mut self) -> Presence<&mut T> {
        match self {
            Presence::Absent => Presence::Absent,
            Presence::Present(value) => Presence::Present(value),
        }
    }

    #[must_use]
    pub fn as_deref(&self) -> Presence<&T::Target>
    where
        T: Deref,
    {
        match self {
            Presence::Absent => Presence::Absent,
            Presence::Present(value) => Presence::Present(Deref::deref(value)),
        }
    }

    #[must_use]
    pub fn as_deref_mut(&mut self) -> Presence<&mut T::Target>
    where
        T: DerefMut,
    {
        match self {
            Presence::Absent => Presence::Absent,
            Presence::Present(value) => Presence::Present(DerefMut::deref_mut(value)),
        }
    }

    #[must_use]
    pub fn map<U, F>(self, function: F) -> Presence<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Presence::Absent => Presence::Absent,
            Presence::Present(value) => Presence::Present(function(value)),
        }
    }

    #[must_use]
    pub fn map_or<U, F>(self, default: U, function: F) -> U
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Presence::Absent => default,
            Presence::Present(value) => function(value),
        }
    }

    #[must_use]
    pub fn map_or_else<U, D, F>(self, default: D, function: F) -> U
    where
        D: FnOnce() -> U,
        F: FnOnce(T) -> U,
    {
        match self {
            Presence::Absent => default(),
            Presence::Present(value) => function(value),
        }
    }

    #[must_use]
    pub fn and_then<U, F>(self, function: F) -> Presence<U>
    where
        F: FnOnce(T) -> Presence<U>,
    {
        match self {
            Presence::Absent => Presence::Absent,
            Presence::Present(value) => function(value),
        }
    }

    #[must_use]
    pub fn or(self, other: Presence<T>) -> Presence<T> {
        match self {
            Presence::Absent => other,
            Presence::Present(value) => Presence::Present(value),
        }
    }

    #[must_use]
    pub fn or_else<F>(self, function: F) -> Presence<T>
    where
        F: FnOnce() -> Presence<T>,
    {
        match self {
            Presence::Absent => function(),
            Presence::Present(value) => Presence::Present(value),
        }
    }

    #[must_use]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Presence::Absent => default,
            Presence::Present(value) => value,
        }
    }

    #[must_use]
    pub fn unwrap_or_else<F>(self, function: F) -> T
    where
        F: FnOnce() -> T,
    {
        match self {
            Presence::Absent => function(),
            Presence::Present(value) => value,
        }
    }

    #[must_use]
    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        match self {
            Presence::Absent => T::default(),
            Presence::Present(value) => value,
        }
    }

    pub fn ok_or<E>(self, error: E) -> Result<T, E> {
        match self {
            Presence::Absent => Result::Err(error),
            Presence::Present(value) => Result::Ok(value),
        }
    }

    pub fn ok_or_else<E, F>(self, error: F) -> Result<T, E>
    where
        F: FnOnce() -> E,
    {
        match self {
            Presence::Absent => Result::Err(error()),
            Presence::Present(value) => Result::Ok(value),
        }
    }
}

impl<T, E> Presence<Result<T, E>> {
    pub fn transpose(self) -> Result<Presence<T>, E> {
        match self {
            Presence::Absent => Result::Ok(Presence::Absent),
            Presence::Present(Result::Ok(value)) => Result::Ok(Presence::Present(value)),
            Presence::Present(Result::Err(error)) => Result::Err(error),
        }
    }
}

impl<T> From<Option<T>> for Presence<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            Option::None => Presence::Absent,
            Option::Some(value) => Presence::Present(value),
        }
    }
}

impl<T> From<Presence<T>> for Option<T> {
    fn from(value: Presence<T>) -> Self {
        match value {
            Presence::Absent => Option::None,
            Presence::Present(value) => Option::Some(value),
        }
    }
}

#[cfg(test)]
#[path = "presence_tests.rs"]
mod presence_tests;
