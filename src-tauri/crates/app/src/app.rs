use std::error::Error;

use crate::port::PortError;
use crate::port::UnitOfWork;
use crate::port::UnitOfWorkFactory;

pub struct App<S, C, I> {
    pub(crate) store: S,
    pub(crate) clock: C,
    pub(crate) id_generator: I,
}

impl<S, C, I> App<S, C, I> {
    pub fn new(store: S, clock: C, id_generator: I) -> Self {
        Self {
            store,
            clock,
            id_generator,
        }
    }
}

impl<S: UnitOfWorkFactory, C, I> App<S, C, I> {
    pub(crate) async fn transact<T, E>(
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
