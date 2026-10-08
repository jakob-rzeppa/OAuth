use redis::AsyncCommands;

use crate::{
    domain::entity::authorization_code::request::AuthorizationRequest,
    error::InternalError,
    persistence::{pars::key, redis::get_redis_connection},
};

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
