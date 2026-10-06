use sqlx::query_as;
use uuid::Uuid;

use crate::{
    domain::entity::user::User,
    persistence::{get_connection, users::UserRow},
};

pub enum FindByIdUserError {
    InvalidData,
    DatabaseError,
}

#[fnmock::fakeable]
pub async fn find_user_by_id(user_id: Uuid) -> Result<Option<User>, FindByIdUserError> {
    let mut conn = get_connection()
        .await
        .map_err(|_| FindByIdUserError::DatabaseError)?;

    let row: Option<UserRow> = query_as!(
        UserRow,
        "SELECT id, user_name, display_name, password_hash, has_temporary_password, roles FROM users WHERE id = $1",
        user_id
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(|error| {
        tracing::error!(?error, "Unknown Database error");
        FindByIdUserError::DatabaseError
    })?;

    if let Some(row) = row {
        Ok(Some(row.into_user().map_err(|error| {
            tracing::error!(?error, "Invalid user row");
            FindByIdUserError::InvalidData
        })?))
    } else {
        Ok(None)
    }
}
