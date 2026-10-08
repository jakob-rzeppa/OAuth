use sqlx::query;

use crate::{error::InternalError, persistence::postgres::get_postgres_connection};

/// Remove (revoke) the access token with the given hash.
/// Removing a token that does not exist is not an error.
#[fnmock::fakeable]
pub async fn remove_access_token(token_hash: &str) -> Result<(), InternalError> {
    let mut conn = get_postgres_connection().await?;

    query!(
        "DELETE FROM access_tokens WHERE token_hash = $1",
        token_hash
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}
