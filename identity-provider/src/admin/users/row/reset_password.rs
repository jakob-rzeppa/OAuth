use axum::extract::Path;

use crate::{
    admin::{
        layout::notice::{ErrorNotice, Notice},
        users::{get_user, parse_user_id, row::UserRowWithNotice},
    },
    application::users::reset_password::{ResetUserPasswordError, reset_user_password},
};

/// POST /admin/users/{user_id}/password/reset
pub async fn reset_password(Path(user_id): Path<String>) -> Result<UserRowWithNotice, ErrorNotice> {
    let user_id = parse_user_id(&user_id)?;

    let temporary_password = reset_user_password(user_id).await.map_err(|e| match e {
        ResetUserPasswordError::UserNotFound => ErrorNotice::user_not_found(),
        ResetUserPasswordError::HashingError | ResetUserPasswordError::DatabaseError => {
            ErrorNotice::internal()
        }
    })?;

    let user = get_user(user_id).await?;
    let notice = Notice::temporary_password(
        format!(
            "Reset the password of {}. Temporary password:",
            user.user_name
        ),
        temporary_password,
    );

    Ok(UserRowWithNotice::new(user, notice))
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use uuid::Uuid;

    use crate::{
        admin::users::testing::{empty, send},
        persistence::users::find_by_id::find_user_by_id_fake,
    };

    #[tokio::test]
    async fn reports_an_unknown_user() {
        find_user_by_id_fake().setup(|_| Ok(None));

        let (status, _, _) = send(empty(
            "POST",
            &format!("/{}/password/reset", Uuid::new_v4()),
        ))
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
