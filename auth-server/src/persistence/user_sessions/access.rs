use redis::{AsyncCommands, Expiry};

use crate::{error::InternalError, persistence::redis::get_redis_connection};

/// Read a serialized session and reset its time to live to `ttl_seconds` (sliding expiration).
/// Returns `None` if the session does not exist or has expired.
#[fnmock::fakeable]
pub async fn access_session(key: &str, ttl_seconds: u64) -> Result<Option<String>, InternalError> {
    let mut conn = get_redis_connection().await?;
    Ok(conn.get_ex(key, Expiry::EX(ttl_seconds)).await?)
}
