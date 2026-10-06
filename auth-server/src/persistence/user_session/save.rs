use redis::AsyncCommands;

use crate::{
    domain::entity::user_session::UserSession,
    persistence::{redis::get_redis_connection, user_session::key},
};

pub enum SaveUserSessionError {
    SerializationError,
    DatabaseError,
}

/// Save a user session under the hash of its session token,
/// to be automatically deleted by redis after `ttl_seconds`.
#[fnmock::mockable]
pub async fn save_user_session(
    session_token_hash: &str,
    session: UserSession,
    ttl_seconds: u64,
) -> Result<(), SaveUserSessionError> {
    let mut conn = get_redis_connection()
        .await
        .map_err(|_| SaveUserSessionError::DatabaseError)?;

    let value = serde_json::to_string(&session).map_err(|error| {
        tracing::error!(?error, "Failed to serialize user session");
        SaveUserSessionError::SerializationError
    })?;

    conn.set_ex::<_, _, ()>(key(session_token_hash), value, ttl_seconds)
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to save user session");
            SaveUserSessionError::DatabaseError
        })
}
