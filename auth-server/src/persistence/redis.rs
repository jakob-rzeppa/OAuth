use crate::config::Config;

#[derive(Debug)]
pub(super) enum RedisError {
    ConnectionError,
}

pub(super) async fn get_redis_connection() -> Result<redis::aio::MultiplexedConnection, RedisError>
{
    let client = redis::Client::open(Config::redis_url()).map_err(|error| {
        tracing::error!(?error, "Failed to create redis client");
        RedisError::ConnectionError
    })?;

    client
        .get_multiplexed_async_connection()
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to connect to redis");
            RedisError::ConnectionError
        })
}
