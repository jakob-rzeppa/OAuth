use redis::AsyncCommands;

use crate::{
    domain::entity::authorization_code::request::AuthorizationRequest,
    persistence::redis::get_redis_connection,
};

pub enum PeekParError {
    InvalidData,
    DatabaseError,
}

/// Read a pushed authorization request by its `request_uri`.
/// Should only used for display purposes, as it does not enforce one-time use.
///
/// Returns `None` if the PAR does not exist or has expired.
pub async fn peek_par(request_uri: &str) -> Result<Option<AuthorizationRequest>, PeekParError> {
    let mut conn = get_redis_connection()
        .await
        .map_err(|_| PeekParError::DatabaseError)?;

    let value: Option<String> = conn
        .get(format!("par:{request_uri}"))
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to peek PAR");
            PeekParError::DatabaseError
        })?;

    value
        .map(|value| {
            serde_json::from_str(&value).map_err(|error| {
                tracing::error!(?error, "Invalid PAR data");
                PeekParError::InvalidData
            })
        })
        .transpose()
}
