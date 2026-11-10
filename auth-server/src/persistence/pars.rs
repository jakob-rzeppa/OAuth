use redis::AsyncCommands;

use crate::{
    domain::authorization_code::request::AuthorizationRequest, error::InternalError,
    persistence::redis::get_redis_connection,
};

fn key(request_uri: &str) -> String {
    format!("par:{request_uri}")
}

/// Save a pushed authorization request, to be automatically deleted by redis after `ttl_seconds`.
#[fnmock::fakeable]
pub async fn save_par(
    request_uri: &str,
    par: AuthorizationRequest,
    ttl_seconds: u64,
) -> Result<(), InternalError> {
    let mut conn = get_redis_connection().await?;
    let value = serde_json::to_string(&par)?;
    Ok(conn
        .set_ex::<_, _, ()>(key(request_uri), value, ttl_seconds)
        .await?)
}

/// Read a pushed authorization request by its `request_uri`.
/// Should only used for display purposes, as it does not enforce one-time use.
///
/// Returns `None` if the PAR does not exist or has expired.
pub async fn find_par(request_uri: &str) -> Result<Option<AuthorizationRequest>, InternalError> {
    let mut conn = get_redis_connection().await?;
    let value: Option<String> = conn.get(key(request_uri)).await?;
    Ok(value
        .map(|value| serde_json::from_str(&value))
        .transpose()?)
}

/// Atomically read and delete a pushed authorization request by its `request_uri`, enforcing one-time use.
/// Returns `None` if the PAR does not exist or has expired.
#[fnmock::mockable]
pub async fn take_par(request_uri: &str) -> Result<Option<AuthorizationRequest>, InternalError> {
    let mut conn = get_redis_connection().await?;
    let value: Option<String> = conn.get_del(key(request_uri)).await?;
    Ok(value
        .map(|value| serde_json::from_str(&value))
        .transpose()?)
}
