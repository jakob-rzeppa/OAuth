use sqlx::query;

use crate::persistence::postgres::get_postgres_connection;

pub enum RemoveAccessTokenError {
    DatabaseError,
}

/// Remove (revoke) the access token with the given hash.
/// Removing a token that does not exist is not an error.
#[fnmock::fakeable]
pub async fn remove_access_token(token_hash: &str) -> Result<(), RemoveAccessTokenError> {
    let mut conn = get_postgres_connection()
        .await
        .map_err(|_| RemoveAccessTokenError::DatabaseError)?;

    query!(
        "DELETE FROM access_tokens WHERE token_hash = $1",
        token_hash
    )
    .execute(&mut *conn)
    .await
    .map_err(|error| {
        tracing::error!(?error, "Unknown Database error");
        RemoveAccessTokenError::DatabaseError
    })?;

    Ok(())
}
