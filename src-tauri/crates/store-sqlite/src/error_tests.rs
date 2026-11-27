use app::port::PortError;

use crate::sqlite::error;
use crate::sqlite::test_db;

#[sqlx::test]
async fn a_database_error_other_than_busy_is_a_backend_failure() {
    let mut db = test_db::open().await;
    let source = sqlx::query("SELECT * FROM missing")
        .execute(&mut db)
        .await
        .expect_err("no such table");
    assert!(matches!(
        error::into_port_error(source),
        PortError::Backend(_)
    ));
}
