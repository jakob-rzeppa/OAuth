use std::sync::LazyLock;

use sqlx::{Pool, Postgres, pool::PoolConnection};

use crate::config::CONFIG;

pub mod roles;
pub mod users;

static DB_POOL: LazyLock<Pool<Postgres>> = LazyLock::new(|| {
    let database_url = CONFIG.database_url();
    Pool::<Postgres>::connect_lazy(database_url).expect("Failed to create database pool")
});

#[derive(Debug)]
pub enum DatabaseError {
    ConnectionError,
}

async fn get_connection() -> Result<PoolConnection<Postgres>, DatabaseError> {
    DB_POOL.acquire().await.map_err(|e| {
        tracing::error!(error = ?e, "Database connection error");
        DatabaseError::ConnectionError
    })
}
