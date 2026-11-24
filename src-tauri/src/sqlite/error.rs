use sqlx::Error;
use sqlx::error::DatabaseError;

use crate::app::port::PortError;

const SQLITE_BUSY: i32 = 5;
const PRIMARY_RESULT_CODE_MASK: i32 = 0xff;

impl From<Error> for PortError {
    fn from(source: Error) -> Self {
        let primary_code = source
            .as_database_error()
            .and_then(DatabaseError::code)
            .and_then(|code| code.parse::<i32>().ok())
            .map(|code| code & PRIMARY_RESULT_CODE_MASK);

        match primary_code {
            Some(SQLITE_BUSY) => Self::Busy(Box::new(source)),
            _ => Self::Backend(Box::new(source)),
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
