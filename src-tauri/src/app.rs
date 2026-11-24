pub(crate) mod assignee;
mod clock;
pub(crate) mod error;
mod id;
pub(crate) mod pagination;
pub(crate) mod port;
pub(crate) mod presence;
#[cfg(test)]
mod testing;
pub(crate) mod todo;

use std::error::Error;

pub use self::assignee::view::AssigneeLifecycleView;
pub use self::assignee::view::AssigneeView;
pub(crate) use self::clock::SystemClock;
pub(crate) use self::id::UuidV7Generator;
pub use self::pagination::ListQuery;
pub use self::pagination::ListQueryError;
pub use self::pagination::NextOffsetOverflow;
pub use self::pagination::Page;
pub use self::todo::view::PreviousTodoLifecycleView;
pub use self::todo::view::TodoLifecycleView;
pub use self::todo::view::TodoStatusView;
pub use self::todo::view::TodoView;
use crate::app::port::PortError;
use crate::app::port::UnitOfWork;
use crate::app::port::UnitOfWorkFactory;

pub(crate) struct App<S, C, I> {
    store: S,
    clock: C,
    id_generator: I,
}

impl<S, C, I> App<S, C, I> {
    pub(crate) fn new(store: S, clock: C, id_generator: I) -> Self {
        Self {
            store,
            clock,
            id_generator,
        }
    }
}

impl<S: UnitOfWorkFactory, C, I> App<S, C, I> {
    async fn transact<T, E>(
        &self,
        work: impl AsyncFnOnce(&mut S::UnitOfWork) -> Result<T, E>,
    ) -> Result<T, E>
    where
        T: Send,
        E: Error + From<PortError> + Send + Sync + 'static,
    {
        let mut uow = self.store.begin().await?;
        let outcome = work(&mut uow).await;
        uow.finalize(outcome).await
    }
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
