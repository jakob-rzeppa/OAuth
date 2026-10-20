use redis::AsyncCommands;

use crate::{
    error::InternalError,
    persistence::{redis::get_redis_connection, self_token::KEY},
};

/// Read the registered self token string.
///
/// Returns `None` if no self token is registered or it has expired.
#[fnmock::fakeable]
pub async fn peek_self_token() -> Result<Option<String>, InternalError> {
    let mut conn = get_redis_connection().await?;
    Ok(conn.get(KEY).await?)
}
