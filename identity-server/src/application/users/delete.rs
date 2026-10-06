use uuid::Uuid;

use crate::persistence::users::{find_by_id::find_user_by_id, remove::remove_user};

pub enum DeleteUserError {
    UserNotFound,
    DatabaseError,
}

pub async fn delete_user(user_id: Uuid) -> Result<(), DeleteUserError> {
    let user = find_user_by_id(user_id)
        .await
        .map_err(|_| DeleteUserError::DatabaseError)?;

    let Some(user) = user else {
        tracing::warn!(%user_id, "delete user: unknown user id");
        return Err(DeleteUserError::UserNotFound);
    };

    remove_user(&user)
        .await
        .map_err(|_| DeleteUserError::DatabaseError)?;

    tracing::info!(%user_id, "user deleted");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::entity::user::User,
        logging::testing::LogCapture,
        persistence::users::{
            find_by_id::find_user_by_id_fake,
            remove::{RemoveUserError, remove_user_fake},
        },
    };

    fn user(id: Uuid) -> User {
        User::new(
            id,
            "alice".to_string(),
            "Alice".to_string(),
            "secret-hash".to_string(),
            false,
            vec![],
        )
        .unwrap()
    }

    #[tokio::test]
    async fn logs_a_deleted_user_by_id_only() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let id = Uuid::new_v4();
        find_user_by_id_fake().setup(move |_| Ok(Some(user(id))));
        remove_user_fake().setup(|_| Ok(()));

        let result = delete_user(id).await;

        assert!(result.is_ok());
        let log = capture.contents();
        assert!(log.contains("INFO"), "{log}");
        assert!(log.contains("user deleted"), "{log}");
        assert!(log.contains(&id.to_string()), "{log}");
        assert!(!log.contains("alice"), "{log}");
    }

    #[tokio::test]
    async fn warns_about_an_unknown_user_id() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let id = Uuid::new_v4();
        find_user_by_id_fake().setup(|_| Ok(None));

        let result = delete_user(id).await;

        assert!(matches!(result, Err(DeleteUserError::UserNotFound)));
        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("unknown user id"), "{log}");
        assert!(log.contains(&id.to_string()), "{log}");
    }

    #[tokio::test]
    async fn does_not_log_a_deletion_when_removing_the_user_fails() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let id = Uuid::new_v4();
        find_user_by_id_fake().setup(move |_| Ok(Some(user(id))));
        remove_user_fake().setup(|_| Err(RemoveUserError::DatabaseError));

        let result = delete_user(id).await;

        assert!(matches!(result, Err(DeleteUserError::DatabaseError)));
        assert!(!capture.contents().contains("user deleted"));
    }
}
