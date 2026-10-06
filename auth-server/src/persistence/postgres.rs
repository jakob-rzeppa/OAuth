use std::sync::LazyLock;

use sqlx::{Pool, Postgres, pool::PoolConnection};

use crate::config;

static DB_POOL: LazyLock<Pool<Postgres>> = LazyLock::new(|| {
    Pool::<Postgres>::connect_lazy(config::database_url()).expect("Failed to create database pool")
});

#[derive(Debug)]
pub enum PostgresError {
    ConnectionError,
}

pub async fn get_postgres_connection() -> Result<PoolConnection<Postgres>, PostgresError> {
    DB_POOL.acquire().await.map_err(|e| {
        tracing::error!(error = ?e, "Database connection error");
        PostgresError::ConnectionError
    })
}
