use redis::AsyncCommands;

use crate::{
    domain::entity::authorization_code::request::AuthorizationRequest,
    error::InternalError,
    persistence::{pars::key, redis::get_redis_connection},
};

/// Read a pushed authorization request by its `request_uri`.
/// Should only used for display purposes, as it does not enforce one-time use.
///
/// Returns `None` if the PAR does not exist or has expired.
pub async fn peek_par(request_uri: &str) -> Result<Option<AuthorizationRequest>, InternalError> {
    let mut conn = get_redis_connection().await?;
    let value: Option<String> = conn.get(key(request_uri)).await?;
    Ok(value
        .map(|value| serde_json::from_str(&value))
        .transpose()?)
}
