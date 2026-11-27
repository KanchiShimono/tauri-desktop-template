use std::error;

use thiserror::Error;

use crate::pagination::NextOffsetOverflow;

type BoxError = Box<dyn error::Error + Send + Sync>;

#[derive(Debug, Error)]
pub enum PortError {
    #[error("the storage is busy")]
    Busy(#[source] BoxError),

    #[error("the port operation failed")]
    Backend(#[source] BoxError),
}

impl From<NextOffsetOverflow> for PortError {
    fn from(source: NextOffsetOverflow) -> Self {
        Self::Backend(Box::new(source))
    }
}
