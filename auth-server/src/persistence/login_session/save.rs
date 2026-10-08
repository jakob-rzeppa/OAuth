use redis::AsyncCommands;

use crate::{
    domain::entity::login_session::LoginSession,
    error::InternalError,
    persistence::{login_session::key, redis::get_redis_connection},
};

/// Save a login session under the hash of its session token,
/// to be automatically deleted by redis after `ttl_seconds`.
#[fnmock::mockable]
pub async fn save_login_session(
    session_token_hash: &str,
    session: LoginSession,
    ttl_seconds: u64,
) -> Result<(), InternalError> {
    let mut conn = get_redis_connection().await?;
    let value = serde_json::to_string(&session)?;
    Ok(conn
        .set_ex::<_, _, ()>(key(session_token_hash), value, ttl_seconds)
        .await?)
}
