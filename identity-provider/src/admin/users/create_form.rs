use serde::Deserialize;
use uuid::Uuid;

use crate::{
    admin::{
        layout::{
            form::HtmlForm,
            notice::{ErrorNotice, Notice},
        },
        users::{get_user, row::UserRowWithNotice},
    },
    application::users::create::{self, CreateUserApplicationError},
};

#[derive(Deserialize)]
pub struct CreateUserForm {
    user_name: String,
    #[serde(default)]
    role_ids: Vec<Uuid>,
}

/// POST /admin/users
pub async fn create_user(
    HtmlForm(CreateUserForm {
        user_name,
        role_ids,
    }): HtmlForm<CreateUserForm>,
) -> Result<UserRowWithNotice, ErrorNotice> {
    let (user_id, temporary_password) =
        create::create_user(user_name, role_ids)
            .await
            .map_err(|e| match e {
                CreateUserApplicationError::InvalidUserName => ErrorNotice::invalid_user_name(),
                CreateUserApplicationError::UserNameAlreadyExists => {
                    ErrorNotice::user_name_already_exists()
                }
                CreateUserApplicationError::PasswordHashingError
                | CreateUserApplicationError::DatabaseError => ErrorNotice::internal(),
            })?;

    let user = get_user(user_id).await?;
    let notice = Notice::temporary_password(
        format!("Created {}. Temporary password:", user.user_name),
        temporary_password,
    );

    Ok(UserRowWithNotice::new(user, notice))
}
