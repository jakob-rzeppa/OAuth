//! A `<tr>` in the `#users` table of the users page.

mod delete;
mod edit_row;
mod reset_password;
mod update;

use askama::Template;
use axum::{extract::Path, routing};

use crate::{
    admin::{
        layout::{
            html::html_response,
            notice::{ErrorNotice, Notice},
        },
        users::{get_user, parse_user_id},
    },
    domain::projection::user::FullUserProjection,
};

#[derive(Template)]
#[template(path = "users/row.html")]
pub struct UserRow {
    user: FullUserProjection,
}

impl UserRow {
    pub fn new(user: FullUserProjection) -> Self {
        Self { user }
    }
}

/// A user row with an out-of-band notice.
#[derive(Template)]
#[template(path = "users/row_with_notice.html")]
pub struct UserRowWithNotice {
    row: UserRow,
    notice: Notice,
}

impl UserRowWithNotice {
    pub fn new(user: FullUserProjection, notice: Notice) -> Self {
        Self {
            row: UserRow::new(user),
            notice,
        }
    }
}

html_response!(UserRow, UserRowWithNotice);

/// /admin/users/{user_id}
pub fn router() -> axum::Router {
    axum::Router::new()
        .route(
            "/",
            routing::get(row)
                .put(update::update_user)
                .delete(delete::delete_user),
        )
        .route("/edit", routing::get(edit_row::edit_row))
        .route(
            "/password/reset",
            routing::post(reset_password::reset_password),
        )
}

/// GET /admin/users/{user_id}
async fn row(Path(user_id): Path<String>) -> Result<UserRow, ErrorNotice> {
    let user = get_user(parse_user_id(&user_id)?).await?;

    Ok(UserRow::new(user))
}
