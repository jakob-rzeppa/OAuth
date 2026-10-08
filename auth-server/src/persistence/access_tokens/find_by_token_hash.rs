use sqlx::query_as;

use crate::{
    domain::entity::access_token::AccessToken,
    error::InternalError,
    persistence::{access_tokens::AccessTokenRow, postgres::get_postgres_connection},
};

/// Find a access token by the hash of the token.
/// Expired tokens are still returned, checking `exp` is up to the caller.
#[fnmock::mockable]
pub async fn find_access_token_by_token_hash(
    token_hash: &str,
) -> Result<Option<AccessToken>, InternalError> {
    let mut conn = get_postgres_connection().await?;

    let row: Option<AccessTokenRow> = query_as!(
        AccessTokenRow,
        "SELECT token_hash, token_type, client_id, sub, iat, exp, scope FROM access_tokens WHERE token_hash = $1",
        token_hash
    )
    .fetch_optional(&mut *conn)
    .await?;

    Ok(row.map(AccessTokenRow::into_access_token))
}
