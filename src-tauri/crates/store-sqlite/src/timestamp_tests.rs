use jiff::Timestamp;
use sqlx::Error;

use crate::test_db;
use crate::timestamp::SqliteTimestamp;

#[sqlx::test]
async fn nanoseconds_around_the_epoch_match_the_persisted_format() {
    let mut db = test_db::open().await;
    for (nanos, bits) in [
        (-1, 0x7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff_u128),
        (0, 0x8000_0000_0000_0000_0000_0000_0000_0000),
        (1, 0x8000_0000_0000_0000_0000_0000_0000_0001),
    ] {
        let time = Timestamp::from_nanosecond(nanos).expect("valid timestamp near the epoch");
        let bytes = bits.to_be_bytes();
        let (storage, encoded, decoded): (String, [u8; 16], SqliteTimestamp) =
            sqlx::query_as("SELECT typeof(?1), ?1, ?2")
                .bind(SqliteTimestamp::from(time))
                .bind(bytes)
                .fetch_one(&mut db)
                .await
                .expect("encode timestamp and decode known bytes");
        assert_eq!(storage, "blob");
        assert_eq!(encoded, bytes);
        assert_eq!(Timestamp::from(decoded), time);
    }
}

#[sqlx::test]
async fn invalid_timestamp_values_fail_to_decode() {
    let mut db = test_db::open().await;
    let mut padded_epoch = [0_u8; 17];
    padded_epoch[0] = 0x80;
    for bytes in [
        &padded_epoch[..15],
        &padded_epoch[..],
        &[0; 16],
        &[u8::MAX; 16],
    ] {
        let result = sqlx::query_scalar::<_, SqliteTimestamp>("SELECT ?")
            .bind(bytes)
            .fetch_one(&mut db)
            .await;
        assert!(
            matches!(result, Err(Error::ColumnDecode { .. })),
            "{bytes:02x?}: {result:?}"
        );
    }
    let text = sqlx::query_scalar::<_, SqliteTimestamp>("SELECT CAST(? AS TEXT)")
        .bind(&padded_epoch[..16])
        .fetch_one(&mut db)
        .await;
    assert!(matches!(text, Err(Error::ColumnDecode { .. })));
}
