use serde::Serialize;
use uuid::Uuid;

use crate::domain::{entity::user::User, projection::role::FullRoleProjection};

#[derive(Serialize, utoipa::ToSchema)]
pub struct FullUserProjection {
    pub id: Uuid,

    pub user_name: String,
    pub display_name: String,

    pub has_temporary_password: bool,

    pub roles: Vec<FullRoleProjection>,
}

impl From<&User> for FullUserProjection {
    fn from(user: &User) -> Self {
        Self {
            id: user.id(),
            user_name: user.user_name().to_string(),
            display_name: user.display_name().to_string(),
            has_temporary_password: user.has_temporary_password(),
            roles: user.roles().iter().map(FullRoleProjection::from).collect(),
        }
    }
}
