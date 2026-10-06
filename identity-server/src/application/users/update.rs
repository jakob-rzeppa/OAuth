use uuid::Uuid;

use crate::persistence::{
    roles::find_by_ids::find_roles_by_ids,
    users::{
        find_by_id::find_user_by_id,
        save::{SaveUserError, save_user},
    },
};

pub enum UpdateUserError {
    UserNotFound,
    InvalidUserName,
    UserNameAlreadyExists,
    DatabaseError,
}

pub async fn update_user(
    user_id: Uuid,
    new_user_name: Option<String>,
    new_display_name: Option<String>,
    role_ids: Option<Vec<Uuid>>,
) -> Result<(), UpdateUserError> {
    let user = find_user_by_id(user_id)
        .await
        .map_err(|_| UpdateUserError::DatabaseError)?;

    let Some(mut user) = user else {
        tracing::warn!(%user_id, "update user: unknown user id");
        return Err(UpdateUserError::UserNotFound);
    };

    if let Some(email) = new_user_name {
        user.set_user_name(email).map_err(|_| {
            tracing::warn!(%user_id, "update user rejected: invalid user name");
            UpdateUserError::InvalidUserName
        })?;
    }

    if let Some(display_name) = new_display_name {
        user.set_display_name(display_name);
    }

    if let Some(role_ids) = role_ids {
        let roles = find_roles_by_ids(&role_ids);

        user.set_roles(roles);
    }

    save_user(&user).await.map_err(|e| match e {
        SaveUserError::UserNotFound => UpdateUserError::DatabaseError,
        SaveUserError::UserNameAlreadyExists => {
            tracing::warn!(%user_id, "update user rejected: user name already exists");
            UpdateUserError::UserNameAlreadyExists
        }
        SaveUserError::DatabaseError => UpdateUserError::DatabaseError,
    })?;

    tracing::info!(%user_id, "user updated");

    Ok(())
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::{
        domain::entity::user::User,
        logging::testing::LogCapture,
        persistence::users::{find_by_id::find_user_by_id_fake, save::save_user_fake},
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
    async fn logs_an_updated_user_by_id_only() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let id = Uuid::new_v4();
        find_user_by_id_fake().setup(move |_| Ok(Some(user(id))));
        save_user_fake().setup(|_| Ok(()));

        let result = update_user(
            id,
            Some("carol".to_string()),
            Some("New Display".to_string()),
            None,
        )
        .await;

        assert!(result.is_ok());
        let log = capture.contents();
        assert!(log.contains("INFO"), "{log}");
        assert!(log.contains("user updated"), "{log}");
        assert!(log.contains(&id.to_string()), "{log}");
        assert!(!log.contains("alice"), "{log}");
        assert!(!log.contains("carol"), "{log}");
        assert!(!log.contains("New Display"), "{log}");
    }

    #[tokio::test]
    async fn warns_about_an_unknown_user_id() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let id = Uuid::new_v4();
        find_user_by_id_fake().setup(|_| Ok(None));

        let result = update_user(id, None, Some("x".to_string()), None).await;

        assert!(matches!(result, Err(UpdateUserError::UserNotFound)));
        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("unknown user id"), "{log}");
        assert!(log.contains(&id.to_string()), "{log}");
        assert!(!log.contains("user updated"), "{log}");
    }

    #[tokio::test]
    async fn warns_about_an_invalid_user_name_without_logging_it() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let id = Uuid::new_v4();
        find_user_by_id_fake().setup(move |_| Ok(Some(user(id))));

        let result = update_user(id, Some("not valid!".to_string()), None, None).await;

        assert!(matches!(result, Err(UpdateUserError::InvalidUserName)));
        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("invalid user name"), "{log}");
        assert!(!log.contains("not valid"), "{log}");
        assert!(!log.contains("user updated"), "{log}");
    }

    #[tokio::test]
    async fn warns_and_does_not_log_an_update_when_the_user_name_is_taken() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let id = Uuid::new_v4();
        find_user_by_id_fake().setup(move |_| Ok(Some(user(id))));
        save_user_fake().setup(|_| Err(SaveUserError::UserNameAlreadyExists));

        let result = update_user(id, Some("carol".to_string()), None, None).await;

        assert!(matches!(
            result,
            Err(UpdateUserError::UserNameAlreadyExists)
        ));
        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("user name already exists"), "{log}");
        assert!(!log.contains("user updated"), "{log}");
        assert!(!log.contains("carol"), "{log}");
    }
}
