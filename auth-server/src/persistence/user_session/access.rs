use redis::{AsyncCommands, Expiry};

use crate::{
    domain::entity::user_session::UserSession,
    error::InternalError,
    persistence::{redis::get_redis_connection, user_session::key},
};

/// Read a user session and reset its time to live to `ttl_seconds` (sliding expiration).
/// Returns `None` if the session does not exist or has expired.
#[fnmock::fakeable]
pub async fn access_user_session(
    session_token_hash: &str,
    ttl_seconds: u64,
) -> Result<Option<UserSession>, InternalError> {
    let mut conn = get_redis_connection().await?;
    let value: Option<String> = conn
        .get_ex(key(session_token_hash), Expiry::EX(ttl_seconds))
        .await?;
    Ok(value
        .map(|value| serde_json::from_str(&value))
        .transpose()?)
}
