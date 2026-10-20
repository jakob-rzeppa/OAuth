use redis::AsyncCommands;

use crate::{
    error::InternalError,
    persistence::{redis::get_redis_connection, self_token::KEY},
};

/// Register the self token string, replacing any previous one, to be automatically deleted by redis after `ttl_seconds`.
#[fnmock::fakeable]
pub async fn register_self_token(token: &str, ttl_seconds: u64) -> Result<(), InternalError> {
    let mut conn = get_redis_connection().await?;
    Ok(conn.set_ex::<_, _, ()>(KEY, token, ttl_seconds).await?)
}
