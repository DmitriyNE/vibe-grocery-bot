use sqlx::{Pool, Sqlite};

#[derive(Clone)]
pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }
}

impl std::ops::Deref for Database {
    type Target = Pool<Sqlite>;
    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
