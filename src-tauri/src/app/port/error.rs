use std::error;

use thiserror::Error;

use crate::app::pagination::NextOffsetOverflow;

type BoxError = Box<dyn error::Error + Send + Sync>;

#[derive(Debug, Error)]
pub(crate) enum PortError {
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
