use uuid::Uuid;

use crate::domain::projection::role::FullRoleProjection;

/// A role as a checkbox in a form.
pub struct RoleOption {
    pub id: Uuid,
    pub name: String,
    pub checked: bool,
}

impl RoleOption {
    /// All `roles`, with the roles in `checked` checked.
    pub fn all(roles: &[FullRoleProjection], checked: &[FullRoleProjection]) -> Vec<Self> {
        roles
            .iter()
            .map(|role| Self {
                id: role.id,
                name: role.name.clone(),
                checked: checked.iter().any(|c| c.id == role.id),
            })
            .collect()
    }
}
