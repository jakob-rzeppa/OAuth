use redis::AsyncCommands;

use crate::{error::InternalError, persistence::redis::get_redis_connection};

/// Save a serialized session under `key`,
/// to be automatically deleted by redis after `ttl_seconds`.
#[fnmock::mockable]
pub async fn save_session(key: &str, value: &str, ttl_seconds: u64) -> Result<(), InternalError> {
    let mut conn = get_redis_connection().await?;
    Ok(conn.set_ex::<_, _, ()>(key, value, ttl_seconds).await?)
}
