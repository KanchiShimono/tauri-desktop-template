use std::error;
use std::future::Future;

use thiserror::Error;

use crate::port::PortError;

#[derive(Debug, Error)]
#[error("rollback failed after an operation error ({operation})")]
struct RollbackFailure<E> {
    operation: E,
    #[source]
    rollback: PortError,
}

pub trait UnitOfWork: Send {
    fn commit(self) -> impl Future<Output = Result<(), PortError>> + Send;

    fn rollback(self) -> impl Future<Output = Result<(), PortError>> + Send;

    fn finalize<T, E>(self, outcome: Result<T, E>) -> impl Future<Output = Result<T, E>> + Send
    where
        Self: Sized,
        T: Send,
        E: error::Error + From<PortError> + Send + Sync + 'static,
    {
        async move {
            match outcome {
                Ok(value) => {
                    self.commit().await?;
                    Ok(value)
                }
                Err(operation) => match self.rollback().await {
                    Ok(()) => Err(operation),
                    Err(rollback) => {
                        let failure = RollbackFailure {
                            operation,
                            rollback,
                        };
                        Err(E::from(PortError::Backend(Box::new(failure))))
                    }
                },
            }
        }
    }
}

pub trait UnitOfWorkFactory: Send + Sync {
    type UnitOfWork: UnitOfWork;

    fn begin(&self) -> impl Future<Output = Result<Self::UnitOfWork, PortError>> + Send;
}

#[cfg(test)]
#[path = "unit_of_work_tests.rs"]
mod tests;
