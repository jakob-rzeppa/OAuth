use crate::{
    application::password::verify::{VerifyPasswordError, verify_password},
    domain::projection::user::FullUserProjection,
    persistence::users::find_by_email::{FindByUserNameUserError, find_user_by_user_name},
};

pub enum AuthenticateUserError {
    InvalidCredentials,
    UserNotFound,
    InternalError,
}

/// Authenticate a user.
///
/// # Returns
///
/// The `FullUserProjection` of the authenticated user and a boolean indicating whether the user must change their password.
pub async fn authenticate_user(
    user_name: &str,
    password: &str,
) -> Result<FullUserProjection, AuthenticateUserError> {
    let user = find_user_by_user_name(user_name)
        .await
        .map_err(|e| match e {
            FindByUserNameUserError::InvalidData => AuthenticateUserError::InternalError,
            FindByUserNameUserError::DatabaseError => AuthenticateUserError::InternalError,
        })?;

    let Some(user) = user else {
        // Expected user behaviour, so INFO. The user name is deliberately not logged.
        tracing::info!("authentication failed: unknown user");
        return Err(AuthenticateUserError::UserNotFound);
    };

    verify_password(password, user.password_hash()).map_err(|e| match e {
        VerifyPasswordError::HashingError => AuthenticateUserError::InternalError,
        VerifyPasswordError::InvalidPassword => {
            tracing::info!(user_id = %user.id(), "authentication failed: invalid password");
            AuthenticateUserError::InvalidCredentials
        }
    })?;

    tracing::info!(user_id = %user.id(), "user authenticated");

    Ok(FullUserProjection::from(&user))
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::{
        application::password::verify::verify_password_fake, domain::entity::user::User,
        logging::testing::LogCapture,
        persistence::users::find_by_email::find_user_by_user_name_fake,
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
    async fn logs_an_authenticated_user_by_id_only() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let id = Uuid::new_v4();
        find_user_by_user_name_fake().setup(move |_| Ok(Some(user(id))));
        verify_password_fake().setup(|_, _| Ok(()));

        let result = authenticate_user("alice", "secret-password").await;

        assert!(result.is_ok());
        let log = capture.contents();
        assert!(log.contains("INFO"), "{log}");
        assert!(log.contains("user authenticated"), "{log}");
        assert!(log.contains(&id.to_string()), "{log}");
        assert!(!log.contains("alice"), "{log}");
        assert!(!log.contains("Alice"), "{log}");
        assert!(!log.contains("secret-password"), "{log}");
        assert!(!log.contains("secret-hash"), "{log}");
    }

    #[tokio::test]
    async fn logs_a_wrong_password_as_info_with_the_user_id() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let id = Uuid::new_v4();
        find_user_by_user_name_fake().setup(move |_| Ok(Some(user(id))));
        verify_password_fake().setup(|_, _| Err(VerifyPasswordError::InvalidPassword));

        let result = authenticate_user("alice", "wrong-password").await;

        assert!(matches!(
            result,
            Err(AuthenticateUserError::InvalidCredentials)
        ));
        let log = capture.contents();
        assert!(log.contains("INFO"), "{log}");
        assert!(
            log.contains("authentication failed: invalid password"),
            "{log}"
        );
        assert!(log.contains(&id.to_string()), "{log}");
        assert!(!log.contains("WARN"), "{log}");
        assert!(!log.contains("user authenticated"), "{log}");
        assert!(!log.contains("wrong-password"), "{log}");
        assert!(!log.contains("alice"), "{log}");
    }

    #[tokio::test]
    async fn logs_an_unknown_user_as_info_without_the_user_name() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        find_user_by_user_name_fake().setup(|_| Ok(None));

        let result = authenticate_user("nobody", "secret-password").await;

        assert!(matches!(result, Err(AuthenticateUserError::UserNotFound)));
        let log = capture.contents();
        assert!(log.contains("INFO"), "{log}");
        assert!(log.contains("authentication failed: unknown user"), "{log}");
        assert!(!log.contains("WARN"), "{log}");
        assert!(!log.contains("nobody"), "{log}");
        assert!(!log.contains("secret-password"), "{log}");
    }
}
