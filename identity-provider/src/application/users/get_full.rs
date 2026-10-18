use uuid::Uuid;

use crate::{
    domain::projection::user::FullUserProjection, persistence::users::find_by_id::find_user_by_id,
};

pub enum GetFullUserError {
    UserNotFound,
    DatabaseError,
}

pub async fn get_full_user_projection(
    user_id: Uuid,
) -> Result<FullUserProjection, GetFullUserError> {
    let user = find_user_by_id(user_id)
        .await
        .map_err(|_| GetFullUserError::DatabaseError)?;

    let Some(user) = user else {
        return Err(GetFullUserError::UserNotFound);
    };

    Ok(FullUserProjection::from(&user))
}
