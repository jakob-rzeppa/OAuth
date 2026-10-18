use crate::{domain::entity::user::User, persistence::get_connection};

pub enum RemoveUserError {
    DatabaseError,
}

#[fnmock::fakeable]
pub async fn remove_user(user: &User) -> Result<(), RemoveUserError> {
    let mut conn = get_connection()
        .await
        .map_err(|_| RemoveUserError::DatabaseError)?;

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user.id())
        .execute(&mut *conn)
        .await
        .map_err(|error| {
            tracing::error!(?error, "Unknown Database error");
            RemoveUserError::DatabaseError
        })?;

    Ok(())
}
