use std::sync::LazyLock;

use sqlx::{Pool, Postgres, pool::PoolConnection};

use crate::{config::Config, error::InternalError};

static DB_POOL: LazyLock<Pool<Postgres>> = LazyLock::new(|| {
    Pool::<Postgres>::connect_lazy(Config::database_url()).expect("Failed to create database pool")
});

pub async fn get_postgres_connection() -> Result<PoolConnection<Postgres>, InternalError> {
    Ok(DB_POOL.acquire().await?)
}
