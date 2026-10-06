use uuid::Uuid;

use crate::{
    application::password::{
        hash::{HashPasswordError, hash_password},
        temporary::generate_temporary_password,
    },
    persistence::users::{
        find_by_id::{FindByIdUserError, find_user_by_id},
        save::{SaveUserError, save_user},
    },
};

pub enum ResetUserPasswordError {
    UserNotFound,
    HashingError,
    DatabaseError,
}

/// Resets the user's password to a new temporary password.
///
/// # Returns
///
/// The new temporary password in plain text.
pub async fn reset_user_password(user_id: Uuid) -> Result<String, ResetUserPasswordError> {
    let user = find_user_by_id(user_id).await.map_err(|e| match e {
        FindByIdUserError::DatabaseError => ResetUserPasswordError::DatabaseError,
        FindByIdUserError::InvalidData => ResetUserPasswordError::DatabaseError,
    })?;

    let Some(mut user) = user else {
        tracing::warn!(%user_id, "reset password: unknown user id");
        return Err(ResetUserPasswordError::UserNotFound);
    };

    // Hash the new temporary password
    let new_password = generate_temporary_password();
    let hashed_password = hash_password(&new_password).map_err(|e| match e {
        HashPasswordError::HashingError => ResetUserPasswordError::HashingError,
    })?;
    user.set_password_hash(hashed_password);

    save_user(&user).await.map_err(|e| match e {
        SaveUserError::DatabaseError => ResetUserPasswordError::DatabaseError,
        SaveUserError::UserNotFound => ResetUserPasswordError::DatabaseError,
        // This case should not happen since we just fetched the user, and don't change the user name.
        SaveUserError::UserNameAlreadyExists => ResetUserPasswordError::DatabaseError,
    })?;

    tracing::info!(%user_id, "user password reset");

    Ok(new_password)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::password::{
            hash::hash_password_fake, temporary::generate_temporary_password_fake,
        },
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

    fn setup_password() {
        generate_temporary_password_fake().setup(|| "tmp-secret".to_string());
        hash_password_fake().setup(|_| Ok("new-hash".to_string()));
    }

    #[tokio::test]
    async fn logs_a_reset_password_without_the_password_or_hash() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let id = Uuid::new_v4();
        setup_password();
        find_user_by_id_fake().setup(move |_| Ok(Some(user(id))));
        save_user_fake().setup(|_| Ok(()));

        let result = reset_user_password(id).await;

        assert!(result.is_ok());
        let log = capture.contents();
        assert!(log.contains("INFO"), "{log}");
        assert!(log.contains("user password reset"), "{log}");
        assert!(log.contains(&id.to_string()), "{log}");
        assert!(!log.contains("tmp-secret"), "{log}");
        assert!(!log.contains("new-hash"), "{log}");
        assert!(!log.contains("old-hash"), "{log}");
        assert!(!log.contains("alice"), "{log}");
    }

    #[tokio::test]
    async fn warns_about_an_unknown_user_id() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let id = Uuid::new_v4();
        find_user_by_id_fake().setup(|_| Ok(None));

        let result = reset_user_password(id).await;

        assert!(matches!(result, Err(ResetUserPasswordError::UserNotFound)));
        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("unknown user id"), "{log}");
        assert!(log.contains(&id.to_string()), "{log}");
    }

    #[tokio::test]
    async fn does_not_log_a_reset_when_saving_fails() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let id = Uuid::new_v4();
        setup_password();
        find_user_by_id_fake().setup(move |_| Ok(Some(user(id))));
        save_user_fake().setup(|_| Err(SaveUserError::DatabaseError));

        let result = reset_user_password(id).await;

        assert!(matches!(result, Err(ResetUserPasswordError::DatabaseError)));
        assert!(!capture.contents().contains("user password reset"));
    }
}
