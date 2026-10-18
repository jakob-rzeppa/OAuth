use uuid::Uuid;

use crate::domain::entity::role::Role;

#[derive(Debug)]
pub enum UserError {
    EmptyId,
    InvalidUserName,
}

pub struct User {
    id: Uuid,

    user_name: String,
    display_name: String,

    password_hash: String,
    has_temporary_password: bool,

    roles: Vec<Role>,
}

impl User {
    pub fn new(
        id: Uuid,
        user_name: String,
        display_name: String,
        password_hash: String,
        has_temporary_password: bool,
        roles: Vec<Role>,
    ) -> Result<Self, UserError> {
        if id.is_nil() {
            return Err(UserError::EmptyId);
        }

        if !is_valid_user_name(&user_name) {
            return Err(UserError::InvalidUserName);
        }

        Ok(Self {
            id,
            user_name,
            display_name,
            password_hash,
            has_temporary_password,
            roles,
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn user_name(&self) -> &str {
        &self.user_name
    }

    pub fn set_user_name(&mut self, user_name: String) -> Result<(), UserError> {
        if !is_valid_user_name(&user_name) {
            return Err(UserError::InvalidUserName);
        }

        self.user_name = user_name;
        Ok(())
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn set_display_name(&mut self, display_name: String) {
        self.display_name = display_name;
    }

    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }

    pub fn set_password_hash(&mut self, password_hash: String) {
        self.password_hash = password_hash;
    }

    pub fn has_temporary_password(&self) -> bool {
        self.has_temporary_password
    }

    pub fn roles(&self) -> &[Role] {
        &self.roles
    }

    pub fn set_roles(&mut self, roles: Vec<Role>) {
        self.roles = roles;
    }
}

fn is_valid_user_name(user_name: &str) -> bool {
    !user_name.trim().is_empty() && user_name.chars().all(|c| c == '.' || c.is_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_nil_id() {
        let result = User::new(
            Uuid::nil(),
            "john.doe".to_string(),
            "John Doe".to_string(),
            "password_hash".to_string(),
            false,
            vec![],
        );
        assert!(matches!(result, Err(UserError::EmptyId)));
    }

    #[test]
    fn creates_user_with_valid_inputs() {
        let id = Uuid::new_v4();
        let user_name = "john.doe".to_string();
        let display_name = "John Doe".to_string();

        let result = User::new(
            id,
            user_name.clone(),
            display_name.clone(),
            "password_hash".to_string(),
            false,
            vec![],
        );

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.id(), id);
        assert_eq!(user.user_name(), &user_name);
        assert_eq!(user.display_name(), &display_name);
    }

    #[test]
    fn rejects_empty_user_name() {
        let result = User::new(
            Uuid::new_v4(),
            "   ".to_string(),
            "John Doe".to_string(),
            "password_hash".to_string(),
            false,
            vec![],
        );

        assert!(matches!(result, Err(UserError::InvalidUserName)));
    }

    #[test]
    fn rejects_user_name_with_invalid_characters() {
        let invalid_names = vec!["john doe", "john@doe", "john/doe", "john_doe"];

        for user_name in invalid_names {
            let result = User::new(
                Uuid::new_v4(),
                user_name.to_string(),
                "John Doe".to_string(),
                "password_hash".to_string(),
                false,
                vec![],
            );

            assert!(
                matches!(result, Err(UserError::InvalidUserName)),
                "should reject user_name: {}",
                user_name
            );
        }
    }

    #[test]
    fn accepts_valid_user_name_formats() {
        let valid_names = vec!["john.doe", "johndoe123", "jörg.müller", "jäger"];

        for user_name in valid_names {
            let result = User::new(
                Uuid::new_v4(),
                user_name.to_string(),
                "John Doe".to_string(),
                "password_hash".to_string(),
                false,
                vec![],
            );

            assert!(result.is_ok(), "should accept user_name: {}", user_name);
        }
    }

    #[test]
    fn set_user_name_updates_user_name_with_valid_input() {
        let mut user = User::new(
            Uuid::new_v4(),
            "old.name".to_string(),
            "John Doe".to_string(),
            "password_hash".to_string(),
            false,
            vec![],
        )
        .unwrap();

        let result = user.set_user_name("new.name".to_string());

        assert!(result.is_ok());
        assert_eq!(user.user_name(), "new.name");
    }

    #[test]
    fn set_user_name_rejects_empty_user_name() {
        let mut user = User::new(
            Uuid::new_v4(),
            "old.name".to_string(),
            "John Doe".to_string(),
            "password_hash".to_string(),
            false,
            vec![],
        )
        .unwrap();

        let result = user.set_user_name("".to_string());

        assert!(matches!(result, Err(UserError::InvalidUserName)));
        assert_eq!(user.user_name(), "old.name");
    }

    #[test]
    fn set_user_name_rejects_user_name_with_invalid_characters() {
        let mut user = User::new(
            Uuid::new_v4(),
            "old.name".to_string(),
            "John Doe".to_string(),
            "password_hash".to_string(),
            false,
            vec![],
        )
        .unwrap();

        let result = user.set_user_name("new name!".to_string());

        assert!(matches!(result, Err(UserError::InvalidUserName)));
        assert_eq!(user.user_name(), "old.name");
    }
}
