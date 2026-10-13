use redis::AsyncCommands;

use crate::{error::InternalError, persistence::redis::get_redis_connection};

/// Atomically read and delete a serialized session, enforcing one-time use.
/// Returns `None` if the session does not exist or has expired.
#[fnmock::fakeable]
pub async fn take_session(key: &str) -> Result<Option<String>, InternalError> {
    let mut conn = get_redis_connection().await?;
    Ok(conn.get_del(key).await?)
}
