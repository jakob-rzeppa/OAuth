use redis::AsyncCommands;

use crate::{
    domain::entity::login_session::LoginSession,
    error::InternalError,
    persistence::{login_session::key, redis::get_redis_connection},
};

/// Atomically read and delete a login session, enforcing one-time use.
/// Returns `None` if the session does not exist or has expired.
#[fnmock::fakeable]
pub async fn take_login_session(
    session_token_hash: &str,
) -> Result<Option<LoginSession>, InternalError> {
    let mut conn = get_redis_connection().await?;
    let value: Option<String> = conn.get_del(key(session_token_hash)).await?;
    Ok(value
        .map(|value| serde_json::from_str(&value))
        .transpose()?)
}
