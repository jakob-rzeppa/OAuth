use sqlx::query;

use crate::{
    domain::entity::user::User,
    persistence::{get_connection, users::role_ids},
};

pub enum SaveUserError {
    UserNotFound,
    UserNameAlreadyExists,
    DatabaseError,
}

const UNIQUE_VIOLATION: &str = "23505";

/// Save changes to a existing user in the database.
/// If the user does not exist, it will throw a error.
#[fnmock::fakeable]
pub async fn save_user(user: &User) -> Result<(), SaveUserError> {
    let mut conn = get_connection()
        .await
        .map_err(|_| SaveUserError::DatabaseError)?;

    let result = query!(
        "UPDATE users SET user_name = $1, display_name = $2, password_hash = $3, has_temporary_password = $4, roles = $5 WHERE id = $6",
        user.user_name(),
        user.display_name(),
        user.password_hash(),
        user.has_temporary_password(),
        &role_ids(user),
        user.id()
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
            SaveUserError::UserNameAlreadyExists
        } else {
            tracing::error!(?error, "Unknown Database error");
            SaveUserError::DatabaseError
        }
    })?;

    if result.rows_affected() == 0 {
        tracing::warn!(user_id = %user.id(), "tried to save changes to an unknown user");
        return Err(SaveUserError::UserNotFound);
    }

    Ok(())
}
