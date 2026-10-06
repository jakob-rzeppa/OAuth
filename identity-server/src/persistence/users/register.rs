use sqlx::query;

use crate::{
    domain::entity::user::User,
    persistence::{get_connection, users::role_ids},
};

pub enum RegisterUserError {
    UserNameAlreadyExists,
    DatabaseError,
}

const UNIQUE_VIOLATION: &str = "23505";

#[fnmock::fakeable]
pub async fn register_user(user: &User) -> Result<(), RegisterUserError> {
    let mut conn = get_connection()
        .await
        .map_err(|_| RegisterUserError::DatabaseError)?;

    query!(
        "INSERT INTO users (id, user_name, display_name, password_hash, has_temporary_password, roles) VALUES ($1, $2, $3, $4, $5, $6)",
        user.id(), user.user_name(), user.display_name(), user.password_hash(), user.has_temporary_password(), &role_ids(user))
        .execute(&mut *conn)
        .await
        .map_err(|error| {
            if error
                .as_database_error()
                .and_then(|db_error| db_error.code())
                .as_deref()
                == Some(UNIQUE_VIOLATION)
            {
                RegisterUserError::UserNameAlreadyExists
            } else {
                tracing::error!(?error, "Unknown Database error");
                RegisterUserError::DatabaseError
            }
        })?;

    Ok(())
}
