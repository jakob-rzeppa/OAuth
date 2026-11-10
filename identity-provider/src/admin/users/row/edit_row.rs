use askama::Template;
use axum::extract::Path;

use crate::{
    admin::{
        layout::{html::html_response, notice::ErrorNotice},
        users::{all_roles, get_user, parse_user_id, role_option::RoleOption},
    },
    domain::projection::{role::FullRoleProjection, user::FullUserProjection},
};

/// The row in its edit state, with inputs instead of text. Replaces the `UserRow`.
#[derive(Template)]
#[template(path = "users/edit_row.html")]
pub struct UserEditRow {
    user: FullUserProjection,
    roles: Vec<RoleOption>,
}

impl UserEditRow {
    pub fn new(user: FullUserProjection, roles: &[FullRoleProjection]) -> Self {
        let roles = RoleOption::all(roles, &user.roles);
        Self { user, roles }
    }
}

html_response!(UserEditRow);

/// GET /admin/users/{user_id}/edit
pub async fn edit_row(Path(user_id): Path<String>) -> Result<UserEditRow, ErrorNotice> {
    let user = get_user(parse_user_id(&user_id)?).await?;

    Ok(UserEditRow::new(user, &all_roles()))
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use uuid::Uuid;

    use crate::{
        admin::users::testing::{empty, send, user},
        persistence::users::find_by_id::find_user_by_id_fake,
    };

    #[tokio::test]
    async fn renders_the_edit_row_with_assigned_roles_checked() {
        let id = Uuid::new_v4();
        find_user_by_id_fake().setup(move |_| Ok(Some(user(id, "alice"))));

        let (status, _, body) = send(empty("GET", &format!("/{id}/edit"))).await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            body.contains("name=\"user_name\" value=\"alice\""),
            "{body}"
        );
        assert!(body.contains("name=\"role_ids\""), "{body}");
        assert!(!body.contains("checked"), "{body}");
    }
}
