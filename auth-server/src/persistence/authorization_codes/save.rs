use redis::AsyncCommands;

use crate::{
    domain::entity::authorization_code::code::AuthorizationCode,
    error::InternalError,
    persistence::{authorization_codes::key, redis::get_redis_connection},
};

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
