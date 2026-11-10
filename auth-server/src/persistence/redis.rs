use crate::{config::Config, error::InternalError};

pub(super) async fn get_redis_connection()
-> Result<redis::aio::MultiplexedConnection, InternalError> {
    let client = redis::Client::open(Config::redis_url())?;
    Ok(client.get_multiplexed_async_connection().await?)
}
