use redis::AsyncCommands;

use crate::{
    domain::entity::login_session::LoginSession,
    persistence::{login_session::key, redis::get_redis_connection},
};

pub enum TakeLoginSessionError {
    InvalidData,
    DatabaseError,
}

/// Atomically read and delete a login session, enforcing one-time use.
/// Returns `None` if the session does not exist or has expired.
#[fnmock::fakeable]
pub async fn take_login_session(
    session_token_hash: &str,
) -> Result<Option<LoginSession>, TakeLoginSessionError> {
    let mut conn = get_redis_connection()
        .await
        .map_err(|_| TakeLoginSessionError::DatabaseError)?;

    let value: Option<String> = conn
        .get_del(key(session_token_hash))
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to take login session");
            TakeLoginSessionError::DatabaseError
        })?;

    value
        .map(|value| {
            serde_json::from_str(&value).map_err(|error| {
                tracing::error!(?error, "Invalid login session data");
                TakeLoginSessionError::InvalidData
            })
        })
        .transpose()
}
