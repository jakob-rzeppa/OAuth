use redis::AsyncCommands;

use crate::{
    domain::entity::authorization_code::code::AuthorizationCode,
    error::InternalError,
    persistence::{authorization_codes::key, redis::get_redis_connection},
};

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
