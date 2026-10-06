use redis::AsyncCommands;

use crate::{
    domain::entity::authorization_code::code::AuthorizationCode,
    persistence::{authorization_codes::key, redis::get_redis_connection},
};

pub enum TakeAuthorizationCodeError {
    InvalidData,
    DatabaseError,
}

/// Atomically read and delete a authorization code, enforcing one-time use.
/// Returns `None` if the code does not exist or has expired.
#[fnmock::fakeable]
pub async fn take_authorization_code(
    request_uri: &str,
) -> Result<Option<AuthorizationCode>, TakeAuthorizationCodeError> {
    let mut conn = get_redis_connection()
        .await
        .map_err(|_| TakeAuthorizationCodeError::DatabaseError)?;

    let value: Option<String> = conn.get_del(key(request_uri)).await.map_err(|error| {
        tracing::error!(?error, "Failed to take authorization code");
        TakeAuthorizationCodeError::DatabaseError
    })?;

    value
        .map(|value| {
            serde_json::from_str(&value).map_err(|error| {
                tracing::error!(?error, "Invalid PAR data");
                TakeAuthorizationCodeError::InvalidData
            })
        })
        .transpose()
}
