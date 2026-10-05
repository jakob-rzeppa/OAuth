use redis::AsyncCommands;

use crate::{
    domain::entity::login_session::LoginSession,
    persistence::{login_session::key, redis::get_redis_connection},
};

pub enum SaveLoginSessionError {
    SerializationError,
    DatabaseError,
}

/// Save a login session under the hash of its session token,
/// to be automatically deleted by redis after `ttl_seconds`.
#[fnmock::mockable]
pub async fn save_login_session(
    session_token_hash: &str,
    session: LoginSession,
    ttl_seconds: u64,
) -> Result<(), SaveLoginSessionError> {
    let mut conn = get_redis_connection()
        .await
        .map_err(|_| SaveLoginSessionError::DatabaseError)?;

    let value = serde_json::to_string(&session).map_err(|error| {
        eprintln!("Failed to serialize login session: {:?}", error);
        SaveLoginSessionError::SerializationError
    })?;

    conn.set_ex::<_, _, ()>(key(session_token_hash), value, ttl_seconds)
        .await
        .map_err(|error| {
            eprintln!("Failed to save login session: {:?}", error);
            SaveLoginSessionError::DatabaseError
        })
}
