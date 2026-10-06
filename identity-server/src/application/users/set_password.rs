use uuid::Uuid;

use crate::{
    application::password::{
        hash::{HashPasswordError, hash_password},
        verify::{VerifyPasswordError, verify_password},
    },
    persistence::users::{
        find_by_id::{FindByIdUserError, find_user_by_id},
        save::{SaveUserError, save_user},
    },
};

pub enum SetUserPasswordError {
    UserNotFound,
    HashingError,
    InvalidCredentials,
    DatabaseError,
}

pub async fn set_user_password(
    user_id: Uuid,
    current_password: &str,
    new_password: &str,
) -> Result<(), SetUserPasswordError> {
    let user = find_user_by_id(user_id).await.map_err(|e| match e {
        FindByIdUserError::DatabaseError => SetUserPasswordError::DatabaseError,
        FindByIdUserError::InvalidData => SetUserPasswordError::DatabaseError,
    })?;

    let Some(mut user) = user else {
        tracing::warn!(%user_id, "set password: unknown user id");
        return Err(SetUserPasswordError::UserNotFound);
    };

    // ==== Verify the old password ====
    verify_password(current_password, user.password_hash()).map_err(|e| match e {
        VerifyPasswordError::HashingError => SetUserPasswordError::HashingError,
        VerifyPasswordError::InvalidPassword => {
            tracing::warn!(%user_id, "set password rejected: wrong current password");
            SetUserPasswordError::InvalidCredentials
        }
    })?;

    // ==== Set the new password ====
    let hashed_password = hash_password(new_password).map_err(|e| match e {
        HashPasswordError::HashingError => SetUserPasswordError::HashingError,
    })?;
    user.set_password_hash(hashed_password);

    save_user(&user).await.map_err(|e| match e {
        SaveUserError::DatabaseError => SetUserPasswordError::DatabaseError,
        SaveUserError::UserNotFound => SetUserPasswordError::DatabaseError,
        // This case should not happen since we just fetched the user, and don't change the user name.
        SaveUserError::UserNameAlreadyExists => SetUserPasswordError::DatabaseError,
    })?;

    tracing::info!(%user_id, "user password changed");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::password::{hash::hash_password_fake, verify::verify_password_fake},
        domain::entity::user::User,
        logging::testing::LogCapture,
        persistence::users::{find_by_id::find_user_by_id_fake, save::save_user_fake},
    };

    fn user(id: Uuid) -> User {
        User::new(
            id,
            "alice".to_string(),
            "Alice".to_string(),
            "old-hash".to_string(),
            false,
            vec![],
        )
        .unwrap()
    }

    #[tokio::test]
    async fn logs_a_changed_password_without_any_password_or_hash() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let id = Uuid::new_v4();
        find_user_by_id_fake().setup(move |_| Ok(Some(user(id))));
        verify_password_fake().setup(|_, _| Ok(()));
        hash_password_fake().setup(|_| Ok("new-hash".to_string()));
        save_user_fake().setup(|_| Ok(()));

        let result = set_user_password(id, "old-password", "new-password").await;

        assert!(result.is_ok());
        let log = capture.contents();
        assert!(log.contains("INFO"), "{log}");
        assert!(log.contains("user password changed"), "{log}");
        assert!(log.contains(&id.to_string()), "{log}");
        assert!(!log.contains("old-password"), "{log}");
        assert!(!log.contains("new-password"), "{log}");
        assert!(!log.contains("old-hash"), "{log}");
        assert!(!log.contains("new-hash"), "{log}");
        assert!(!log.contains("alice"), "{log}");
    }

    #[tokio::test]
    async fn warns_about_a_wrong_current_password_and_changes_nothing() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let id = Uuid::new_v4();
        find_user_by_id_fake().setup(move |_| Ok(Some(user(id))));
        verify_password_fake().setup(|_, _| Err(VerifyPasswordError::InvalidPassword));
        // `hash_password` and `save_user` are not faked: reaching them would fail the test.

        let result = set_user_password(id, "wrong-password", "new-password").await;

        assert!(matches!(
            result,
            Err(SetUserPasswordError::InvalidCredentials)
        ));
        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("wrong current password"), "{log}");
        assert!(log.contains(&id.to_string()), "{log}");
        assert!(!log.contains("user password changed"), "{log}");
        assert!(!log.contains("wrong-password"), "{log}");
        assert!(!log.contains("new-password"), "{log}");
    }

    #[tokio::test]
    async fn warns_about_an_unknown_user_id() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let id = Uuid::new_v4();
        find_user_by_id_fake().setup(|_| Ok(None));

        let result = set_user_password(id, "old-password", "new-password").await;

        assert!(matches!(result, Err(SetUserPasswordError::UserNotFound)));
        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("unknown user id"), "{log}");
        assert!(log.contains(&id.to_string()), "{log}");
    }
}
