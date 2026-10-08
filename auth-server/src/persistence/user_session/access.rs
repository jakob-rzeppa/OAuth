use redis::{AsyncCommands, Expiry};

use crate::{
    domain::entity::user_session::UserSession,
    persistence::{redis::get_redis_connection, user_session::key},
};

pub enum AccessUserSessionError {
    InvalidData,
    DatabaseError,
}

/// Read a user session and reset its time to live to `ttl_seconds` (sliding expiration).
/// Returns `None` if the session does not exist or has expired.
#[fnmock::fakeable]
pub async fn access_user_session(
    session_token_hash: &str,
    ttl_seconds: u64,
) -> Result<Option<UserSession>, AccessUserSessionError> {
    let mut conn = get_redis_connection()
        .await
        .map_err(|_| AccessUserSessionError::DatabaseError)?;

    let value: Option<String> = conn
        .get_ex(key(session_token_hash), Expiry::EX(ttl_seconds))
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to access user session");
            AccessUserSessionError::DatabaseError
        })?;

    value
        .map(|value| {
            serde_json::from_str(&value).map_err(|error| {
                tracing::error!(?error, "Invalid user session data");
                AccessUserSessionError::InvalidData
            })
        })
        .transpose()
}
