use crate::app::port::PortError;
use crate::sqlite::test_db;

#[sqlx::test]
async fn a_database_error_other_than_busy_is_a_backend_failure() {
    let mut db = test_db::open().await;
    let error = sqlx::query("SELECT * FROM missing")
        .execute(&mut db)
        .await
        .expect_err("no such table");
    assert!(matches!(PortError::from(error), PortError::Backend(_)));
}
