//! The /admin/users page.

mod create_form;
mod role_option;
mod row;
#[cfg(test)]
mod testing;

use askama::Template;
use axum::routing;
use uuid::Uuid;

use crate::{
    admin::{
        layout::{html::html_response, notice::ErrorNotice},
        users::{role_option::RoleOption, row::UserRow},
    },
    application::users::get_full::{GetFullUserError, get_full_user_projection},
    domain::projection::{role::FullRoleProjection, user::FullUserProjection},
    persistence::{roles::find_all::find_all_roles, users::find_all::find_all_users},
};

/// The users table with the form to create a user.
#[derive(Template)]
#[template(path = "users/page.html")]
pub struct UsersPage {
    rows: Vec<UserRow>,
    roles: Vec<RoleOption>,
}

impl UsersPage {
    pub fn new(users: Vec<FullUserProjection>, roles: &[FullRoleProjection]) -> Self {
        Self {
            rows: users.into_iter().map(UserRow::new).collect(),
            roles: RoleOption::all(roles, &[]),
        }
    }
}

html_response!(UsersPage);

/// /admin/users
pub fn router() -> axum::Router {
    axum::Router::new()
        .route("/", routing::get(users_page).post(create_form::create_user))
        .nest("/{user_id}", row::router())
}

/// GET /admin/users
async fn users_page() -> Result<UsersPage, ErrorNotice> {
    let users = find_all_users()
        .await
        .map_err(|_| ErrorNotice::internal())?;
    let mut users: Vec<FullUserProjection> = users.iter().map(FullUserProjection::from).collect();
    users.sort_by(|a, b| a.user_name.cmp(&b.user_name));

    Ok(UsersPage::new(users, &all_roles()))
}

fn parse_user_id(user_id: &str) -> Result<Uuid, ErrorNotice> {
    Uuid::parse_str(user_id).map_err(|_| ErrorNotice::invalid_user_id())
}

async fn get_user(user_id: Uuid) -> Result<FullUserProjection, ErrorNotice> {
    get_full_user_projection(user_id)
        .await
        .map_err(|e| match e {
            GetFullUserError::UserNotFound => ErrorNotice::user_not_found(),
            GetFullUserError::DatabaseError => ErrorNotice::internal(),
        })
}

fn all_roles() -> Vec<FullRoleProjection> {
    find_all_roles()
        .iter()
        .map(FullRoleProjection::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_the_users_escaped() {
        let user = FullUserProjection {
            id: Uuid::new_v4(),
            user_name: "alice".to_string(),
            display_name: "<script>".to_string(),
            has_temporary_password: false,
            roles: vec![],
        };

        let body = UsersPage::new(vec![user], &[]).render().unwrap();

        assert!(body.contains("alice"), "{body}");
        assert!(body.contains("&#60;script&#62;"), "{body}");
        assert!(!body.contains("<script>"), "{body}");
    }
}
