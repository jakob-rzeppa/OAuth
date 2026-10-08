use sqlx::query;

use crate::{
    domain::entity::access_token::AccessToken, error::InternalError,
    persistence::postgres::get_postgres_connection,
};

#[fnmock::fakeable]
pub async fn register_access_token(access_token: &AccessToken) -> Result<(), InternalError> {
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
