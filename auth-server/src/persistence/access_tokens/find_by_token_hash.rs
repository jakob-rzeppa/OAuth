use sqlx::query_as;

use crate::{
    domain::entity::access_token::AccessToken,
    persistence::{access_tokens::AccessTokenRow, postgres::get_postgres_connection},
};

pub enum FindByTokenHashAccessTokenError {
    DatabaseError,
}

/// Find a access token by the hash of the token.
/// Expired tokens are still returned, checking `exp` is up to the caller.
#[fnmock::mockable]
pub async fn find_access_token_by_token_hash(
    token_hash: &str,
) -> Result<Option<AccessToken>, FindByTokenHashAccessTokenError> {
    let mut conn = get_postgres_connection()
        .await
        .map_err(|_| FindByTokenHashAccessTokenError::DatabaseError)?;

    let row: Option<AccessTokenRow> = query_as!(
        AccessTokenRow,
        "SELECT token_hash, token_type, client_id, iat, exp, scope FROM access_tokens WHERE token_hash = $1",
        token_hash
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(|error| {
        tracing::error!(?error, "Unknown Database error");
        FindByTokenHashAccessTokenError::DatabaseError
    })?;

    Ok(row.map(AccessTokenRow::into_access_token))
}
