use axum::extract::Path;

use crate::{
    admin::{
        layout::notice::{ErrorNotice, Notice},
        users::parse_user_id,
    },
    application::users::delete::{self, DeleteUserError},
};

/// DELETE /admin/users/{user_id}
pub async fn delete_user(Path(user_id): Path<String>) -> Result<Notice, ErrorNotice> {
    let user_id = parse_user_id(&user_id)?;

    delete::delete_user(user_id).await.map_err(|e| match e {
        DeleteUserError::UserNotFound => ErrorNotice::user_not_found(),
        DeleteUserError::DatabaseError => ErrorNotice::internal(),
    })?;

    Ok(Notice::success("Deleted the user.".to_string()))
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use uuid::Uuid;

    use crate::{
        admin::users::testing::{empty, send, user},
        persistence::users::{find_by_id::find_user_by_id_fake, remove::remove_user_fake},
    };

    #[tokio::test]
    async fn deletes_a_user_with_only_an_out_of_band_notice() {
        let id = Uuid::new_v4();
        find_user_by_id_fake().setup(move |_| Ok(Some(user(id, "alice"))));
        remove_user_fake().setup(|_| Ok(()));

        let (status, _, body) = send(empty("DELETE", &format!("/{id}"))).await;

        assert_eq!(status, StatusCode::OK);
        assert!(!body.contains("<tr"), "{body}");
        assert!(body.contains("hx-swap-oob=\"true\""), "{body}");
    }

    #[tokio::test]
    async fn reports_an_unknown_user() {
        find_user_by_id_fake().setup(|_| Ok(None));

        let (status, _, _) = send(empty("DELETE", &format!("/{}", Uuid::new_v4()))).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn rejects_an_invalid_user_id() {
        let (status, _, body) = send(empty("DELETE", "/not-a-uuid")).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.contains("The user ID is invalid."), "{body}");
    }
}
