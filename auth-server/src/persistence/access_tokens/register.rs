use sqlx::query;

use crate::{
    domain::entity::access_token::AccessToken, persistence::postgres::get_postgres_connection,
};

pub enum RegisterAccessTokenError {
    TokenAlreadyExists,
    DatabaseError,
}

const UNIQUE_VIOLATION: &str = "23505";

#[fnmock::fakeable]
pub async fn register_access_token(
    access_token: &AccessToken,
) -> Result<(), RegisterAccessTokenError> {
    let mut conn = get_postgres_connection()
        .await
        .map_err(|_| RegisterAccessTokenError::DatabaseError)?;

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
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .and_then(|db_error| db_error.code())
            .as_deref()
            == Some(UNIQUE_VIOLATION)
        {
            RegisterAccessTokenError::TokenAlreadyExists
        } else {
            tracing::error!(?error, "Unknown Database error");
            RegisterAccessTokenError::DatabaseError
        }
    })?;

    Ok(())
}
