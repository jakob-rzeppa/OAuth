use uuid::Uuid;

use crate::{domain::entity::role::Role, persistence::roles::store::roles};

pub fn find_role_by_id(role_id: &Uuid) -> Option<Role> {
    roles().iter().find(|role| role.id().eq(role_id)).cloned()
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use uuid::Uuid;

    use crate::persistence::roles::store::roles_fake;

    use super::*;

    static TEST_ROLES: LazyLock<Vec<Role>> = LazyLock::new(|| {
        vec![
            Role::new(
                Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                "Admin".to_string(),
                "Administrator role".to_string(),
            )
            .unwrap(),
            Role::new(
                Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
                "User".to_string(),
                "Regular user role".to_string(),
            )
            .unwrap(),
        ]
    });

    #[test]
    fn test_find_role_by_id() {
        roles_fake().setup(|| &TEST_ROLES);

        let role =
            find_role_by_id(&Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap());

        assert!(role.is_some());
        let role = role.unwrap();
        assert_eq!(
            role.id(),
            &Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
        );
        assert_eq!(role.name(), "Admin");
        assert_eq!(role.description(), "Administrator role");
    }
}
