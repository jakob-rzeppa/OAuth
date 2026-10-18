use std::sync::LazyLock;

use crate::domain::entity::role::Role;

#[derive(serde::Deserialize)]
struct RoleRow {
    id: String,
    name: String,
    description: String,
}

#[derive(serde::Deserialize)]
struct RoleStore {
    roles: Vec<RoleRow>,
}

static ROLES: LazyLock<Vec<Role>> = LazyLock::new(|| {
    let roles_yaml_data =
        std::fs::read_to_string("seed/roles.yaml").expect("Failed to read roles.yaml");
    let roles_data: RoleStore =
        serde_yaml::from_str(&roles_yaml_data).expect("Failed to parse roles.yaml");

    roles_data
        .roles
        .into_iter()
        .map(|row| {
            Role::new(
                uuid::Uuid::parse_str(&row.id).expect("Invalid UUID in roles.yaml"),
                row.name,
                row.description,
            )
            .expect("Failed to create Role from roles.yaml")
        })
        .collect()
});

#[fnmock::fakeable]
pub(super) fn roles() -> &'static [Role] {
    &ROLES
}
