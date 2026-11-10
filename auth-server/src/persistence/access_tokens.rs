use chrono::{DateTime, Utc};
use sqlx::{prelude::FromRow, query, query_as};
use uuid::Uuid;

use crate::{
    domain::access_token::AccessToken, error::InternalError,
    persistence::postgres::get_postgres_connection,
};

#[derive(FromRow)]
struct AccessTokenRow {
    token_hash: String,
    token_type: String,
    client_id: Uuid,
    sub: Option<Uuid>,
    iat: DateTime<Utc>,
    exp: DateTime<Utc>,
    scope: String,
}

impl AccessTokenRow {
    fn into_access_token(self) -> AccessToken {
        AccessToken::new(
            self.token_hash,
            self.token_type,
            self.client_id,
            self.sub,
            self.iat,
            self.exp,
            self.scope,
        )
    }
}

#[fnmock::fakeable]
pub async fn save_access_token(access_token: &AccessToken) -> Result<(), InternalError> {
    let mut conn = get_postgres_connection().await?;

    query!(
        "INSERT INTO access_tokens (token_hash, token_type, client_id, sub, iat, exp, scope) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        access_token.token_hash(),
        access_token.token_type(),
        access_token.client_id(),
        access_token.sub(),
        access_token.iat(),
        access_token.exp(),
        access_token.scope()
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Find a access token by the hash of the token.
/// Expired tokens are still returned, checking `exp` is up to the caller.
#[fnmock::mockable]
pub async fn find_access_token(token_hash: &str) -> Result<Option<AccessToken>, InternalError> {
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

/// Delete (revoke) the access token with the given hash.
/// Deleting a token that does not exist is not an error.
#[fnmock::fakeable]
pub async fn delete_access_token(token_hash: &str) -> Result<(), InternalError> {
    let mut conn = get_postgres_connection().await?;

    query!(
        "DELETE FROM access_tokens WHERE token_hash = $1",
        token_hash
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}
