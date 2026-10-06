use redis::AsyncCommands;

use crate::{
    domain::entity::authorization_code::request::AuthorizationRequest,
    persistence::redis::get_redis_connection,
};

pub enum TakeParError {
    InvalidData,
    DatabaseError,
}

/// Atomically read and delete a pushed authorization request by its `request_uri`, enforcing one-time use.
/// Returns `None` if the PAR does not exist or has expired.
#[fnmock::mockable]
pub async fn take_par(request_uri: &str) -> Result<Option<AuthorizationRequest>, TakeParError> {
    let mut conn = get_redis_connection()
        .await
        .map_err(|_| TakeParError::DatabaseError)?;

    let value: Option<String> =
        conn.get_del(format!("par:{request_uri}"))
            .await
            .map_err(|error| {
                tracing::error!(?error, "Failed to take PAR");
                TakeParError::DatabaseError
            })?;

    value
        .map(|value| {
            serde_json::from_str(&value).map_err(|error| {
                tracing::error!(?error, "Invalid PAR data");
                TakeParError::InvalidData
            })
        })
        .transpose()
}
