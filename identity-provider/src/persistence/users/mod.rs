use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::{domain::entity::user::User, persistence::roles::find_by_ids::find_roles_by_ids};

pub mod find_all;
pub mod find_by_email;
pub mod find_by_id;
pub mod register;
pub mod remove;
pub mod save;

#[derive(FromRow)]
struct UserRow {
    id: Uuid,
    user_name: String,
    display_name: String,
    password_hash: String,
    has_temporary_password: bool,
    /// The ids of the roles of the user. The roles themselves live in the role store.
    roles: Vec<Uuid>,
}

#[derive(Debug)]
enum UserRowError {
    /// The row references a role id that the role store does not know.
    ViolatedUserInvariant,
}

impl UserRow {
    fn into_user(self) -> Result<User, UserRowError> {
        let roles = find_roles_by_ids(&self.roles);

        User::new(
            self.id,
            self.user_name,
            self.display_name,
            self.password_hash,
            self.has_temporary_password,
            roles,
        )
        .map_err(|_| UserRowError::ViolatedUserInvariant)
    }
}

/// The role ids of a user, ready to be bound to a `UUID[]` column.
fn role_ids(user: &User) -> Vec<Uuid> {
    user.roles().iter().map(|role| role.id().clone()).collect()
}
