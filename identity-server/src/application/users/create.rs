use uuid::Uuid;

use crate::{
    application::password::{hash::hash_password, temporary::generate_temporary_password},
    domain::entity::user::{User, UserError},
    persistence::{
        roles::find_by_ids::find_roles_by_ids,
        users::register::{RegisterUserError, register_user},
    },
};

pub enum CreateUserApplicationError {
    InvalidUserName,
    UserNameAlreadyExists,
    PasswordHashingError,
    DatabaseError,
}

/// Creates a new user with the given user_name and display_name and returns the user's ID and a temporary password if successful.
pub async fn create_user(
    user_name: String,
    role_ids: Vec<Uuid>,
) -> Result<(Uuid, String), CreateUserApplicationError> {
    let has_temporary_password = true;
    let temporary_password = generate_temporary_password();
    let password_hash = hash_password(&temporary_password)
        .map_err(|_| CreateUserApplicationError::PasswordHashingError)?;

    let display_name = build_initial_display_name(&user_name);

    let roles = find_roles_by_ids(&role_ids);

    let user = User::new(
        Uuid::new_v4(),
        user_name,
        display_name,
        password_hash,
        has_temporary_password,
        roles,
    )
    .map_err(|e| match e {
        UserError::EmptyId => unreachable!("Uuid::new_v4() does not create a nil Uuid."),
        UserError::InvalidUserName => {
            tracing::warn!("create user rejected: invalid user name");
            CreateUserApplicationError::InvalidUserName
        }
    })?;

    register_user(&user).await.map_err(|err| match err {
        RegisterUserError::UserNameAlreadyExists => {
            tracing::warn!("create user rejected: user name already exists");
            CreateUserApplicationError::UserNameAlreadyExists
        }
        RegisterUserError::DatabaseError => CreateUserApplicationError::DatabaseError,
    })?;

    tracing::info!(user_id = %user.id(), "user created");

    Ok((user.id(), temporary_password))
}

fn build_initial_display_name(user_name: &str) -> String {
    let parts: Vec<&str> = user_name.split('.').collect();

    parts
        .into_iter()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::password::{
            hash::hash_password_fake, temporary::generate_temporary_password_fake,
        },
        logging::testing::LogCapture,
        persistence::{
            roles::find_by_ids::find_roles_by_ids_fake,
            users::register::{RegisterUserError, register_user_fake},
        },
    };

    fn setup_password_and_roles() {
        generate_temporary_password_fake().setup(|| "tmp-secret".to_string());
        hash_password_fake().setup(|_| Ok("secret-hash".to_string()));
        find_roles_by_ids_fake().setup(|_| vec![]);
    }

    #[tokio::test]
    async fn logs_a_created_user_by_id_only() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        setup_password_and_roles();
        register_user_fake().setup(|_| Ok(()));

        let result = create_user("bob.smith".to_string(), vec![]).await;

        let Ok((id, _)) = result else {
            panic!("expected a created user");
        };
        let log = capture.contents();
        assert!(log.contains("INFO"), "{log}");
        assert!(log.contains("user created"), "{log}");
        assert!(log.contains(&id.to_string()), "{log}");
        assert!(!log.contains("tmp-secret"), "{log}");
        assert!(!log.contains("secret-hash"), "{log}");
        assert!(!log.contains("bob"), "{log}");
        assert!(!log.contains("Bob"), "{log}");
    }

    #[tokio::test]
    async fn warns_and_does_not_log_a_created_user_when_the_user_name_exists() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        setup_password_and_roles();
        register_user_fake().setup(|_| Err(RegisterUserError::UserNameAlreadyExists));

        let result = create_user("bob.smith".to_string(), vec![]).await;

        assert!(matches!(
            result,
            Err(CreateUserApplicationError::UserNameAlreadyExists)
        ));
        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("user name already exists"), "{log}");
        assert!(!log.contains("user created"), "{log}");
        assert!(!log.contains("bob"), "{log}");
    }

    #[tokio::test]
    async fn warns_about_an_invalid_user_name_before_touching_the_database() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        // `register_user` is not faked: reaching it would fail the test.
        setup_password_and_roles();

        let result = create_user("not valid!".to_string(), vec![]).await;

        assert!(matches!(
            result,
            Err(CreateUserApplicationError::InvalidUserName)
        ));
        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("invalid user name"), "{log}");
        assert!(!log.contains("not valid"), "{log}");
    }
}
