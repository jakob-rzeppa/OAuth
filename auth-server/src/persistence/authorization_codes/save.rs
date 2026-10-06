use redis::AsyncCommands;

use crate::{
    domain::entity::authorization_code::code::AuthorizationCode,
    persistence::{authorization_codes::key, redis::get_redis_connection},
};

pub enum SaveAuthorizationCodeError {
    SerializationError,
    DatabaseError,
}

/// Save a authorization code, to be automatically deleted by redis after `ttl_seconds`.
#[fnmock::mockable]
pub async fn save_authorization_code(
    code: AuthorizationCode,
    ttl_seconds: u64,
) -> Result<(), SaveAuthorizationCodeError> {
    let mut conn = get_redis_connection()
        .await
        .map_err(|_| SaveAuthorizationCodeError::DatabaseError)?;

    let value = serde_json::to_string(&code).map_err(|error| {
        tracing::error!(?error, "Failed to serialize PAR");
        SaveAuthorizationCodeError::SerializationError
    })?;

    conn.set_ex::<_, _, ()>(key(code.code()), value, ttl_seconds)
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to save authorization code");
            SaveAuthorizationCodeError::DatabaseError
        })
}
