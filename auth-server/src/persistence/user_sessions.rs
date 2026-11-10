use redis::{AsyncCommands, Expiry};

use crate::{error::InternalError, persistence::redis::get_redis_connection};

/// Save a serialized session under `key`,
/// to be automatically deleted by redis after `ttl_seconds`.
#[fnmock::mockable]
pub async fn save_session(key: &str, value: &str, ttl_seconds: u64) -> Result<(), InternalError> {
    let mut conn = get_redis_connection().await?;
    Ok(conn.set_ex::<_, _, ()>(key, value, ttl_seconds).await?)
}

/// Read a serialized session and reset its time to live to `ttl_seconds` (sliding expiration).
/// Returns `None` if the session does not exist or has expired.
#[fnmock::fakeable]
pub async fn renew_session(key: &str, ttl_seconds: u64) -> Result<Option<String>, InternalError> {
    let mut conn = get_redis_connection().await?;
    Ok(conn.get_ex(key, Expiry::EX(ttl_seconds)).await?)
}
