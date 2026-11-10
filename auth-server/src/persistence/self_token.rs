use redis::AsyncCommands;

use crate::{error::InternalError, persistence::redis::get_redis_connection};

const KEY: &str = "self_token";

/// Save the self token string, replacing any previous one, to be automatically deleted by redis after `ttl_seconds`.
#[fnmock::fakeable]
pub async fn save_self_token(token: &str, ttl_seconds: u64) -> Result<(), InternalError> {
    let mut conn = get_redis_connection().await?;
    Ok(conn.set_ex::<_, _, ()>(KEY, token, ttl_seconds).await?)
}

/// Read the saved self token string.
///
/// Returns `None` if no self token is saved or it has expired.
#[fnmock::fakeable]
pub async fn find_self_token() -> Result<Option<String>, InternalError> {
    let mut conn = get_redis_connection().await?;
    Ok(conn.get(KEY).await?)
}
