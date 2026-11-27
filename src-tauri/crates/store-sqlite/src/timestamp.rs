use jiff::Timestamp;
use macros::From;
use macros::Into;
use sqlx::Decode;
use sqlx::Encode;
use sqlx::Sqlite;
use sqlx::Type;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::sqlite::SqliteArgumentsBuffer;
use sqlx::sqlite::SqliteTypeInfo;
use sqlx::sqlite::SqliteValueRef;

#[derive(Clone, Copy, Debug, Eq, From, Into, PartialEq)]
pub(super) struct SqliteTimestamp(Timestamp);

impl Type<Sqlite> for SqliteTimestamp {
    fn type_info() -> SqliteTypeInfo {
        <[u8; 16] as Type<Sqlite>>::type_info()
    }
}

impl Encode<'_, Sqlite> for SqliteTimestamp {
    fn encode_by_ref(&self, buf: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        // Flipping the sign bit puts pre-epoch values first in SQLite's bytewise order.
        let bytes = (self.0.as_nanosecond() ^ i128::MIN).to_be_bytes();
        Encode::<Sqlite>::encode(bytes, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for SqliteTimestamp {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let bytes = <[u8; 16] as Decode<'r, Sqlite>>::decode(value)?;
        let nanos = i128::from_be_bytes(bytes) ^ i128::MIN;
        Ok(Self(Timestamp::from_nanosecond(nanos)?))
    }
}

#[cfg(test)]
#[path = "timestamp_tests.rs"]
mod tests;
