use sqlx::SqlitePool;

#[derive(Clone, Debug)]
pub(crate) struct Store(SqlitePool);

impl Store {
    pub const fn new(pool: SqlitePool) -> Self {
        Self(pool)
    }

    pub(crate) const fn pool(&self) -> &SqlitePool {
        &self.0
    }
}
