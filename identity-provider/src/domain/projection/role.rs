use serde::Serialize;
use uuid::Uuid;

use crate::domain::entity::role::Role;

#[derive(Serialize, utoipa::ToSchema)]
pub struct FullRoleProjection {
    pub id: Uuid,
    pub name: String,
    pub description: String,
}

impl From<&Role> for FullRoleProjection {
    fn from(role: &Role) -> Self {
        Self {
            id: role.id().clone(),
            name: role.name().to_string(),
            description: role.description().to_string(),
        }
    }
}
