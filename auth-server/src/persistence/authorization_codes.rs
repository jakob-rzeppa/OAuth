use redis::AsyncCommands;

use crate::{
    domain::authorization_code::code::AuthorizationCode, error::InternalError,
    persistence::redis::get_redis_connection,
};

fn key(request_uri: &str) -> String {
    format!("authorization_code:{request_uri}")
}

/// Save a authorization code, to be automatically deleted by redis after `ttl_seconds`.
#[fnmock::mockable]
pub async fn save_authorization_code(
    code: AuthorizationCode,
    ttl_seconds: u64,
) -> Result<(), InternalError> {
    let mut conn = get_redis_connection().await?;
    let value = serde_json::to_string(&code)?;
    Ok(conn
        .set_ex::<_, _, ()>(key(code.code()), value, ttl_seconds)
        .await?)
}

/// Atomically read and delete a authorization code, enforcing one-time use.
/// Returns `None` if the code does not exist or has expired.
#[fnmock::fakeable]
pub async fn take_authorization_code(
    code: &str,
) -> Result<Option<AuthorizationCode>, InternalError> {
    let mut conn = get_redis_connection().await?;
    let value: Option<String> = conn.get_del(key(code)).await?;
    Ok(value
        .map(|value| serde_json::from_str(&value))
        .transpose()?)
}
