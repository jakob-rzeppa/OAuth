use askama::Template;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{endpoints::login::error::LoginError, util::html::render_html};

/// Builds the login form.
/// `user_name` pre-fills the user name field, e.g. after a failed login attempt.
pub(super) async fn render_login_page(
    return_to: String,
    user_name: Option<String>,
    error_message: Option<String>,
) -> Result<LoginPageResponse, LoginError> {
    Ok(LoginPageResponse {
        return_to,
        user_name,
        error_message,
    })
}

pub struct LoginPageResponse {
    pub return_to: String,

    // Pre-filled username
    pub user_name: Option<String>,

    // On error
    pub error_message: Option<String>,
}

#[derive(Template)]
#[template(path = "pages/login.html")]
struct LoginPage {
    return_to: String,

    // Pre-filled username
    user_name: Option<String>,

    // On error
    error_message: Option<String>,
}

impl IntoResponse for LoginPageResponse {
    fn into_response(self) -> Response {
        let Self {
            return_to,
            user_name,
            error_message,
        } = self;

        let page = LoginPage {
            return_to,
            user_name,
            error_message,
        };

        render_html(&page, StatusCode::OK).into_response()
    }
}

/// The form forcing a user who logged in with a temporary password to choose a new one.
pub struct ChangePasswordPageResponse {
    pub return_to: String,
    pub user_name: String,

    // On error
    pub error_message: Option<String>,
}

#[derive(Template)]
#[template(path = "pages/change_password.html")]
struct ChangePasswordPage {
    return_to: String,
    user_name: String,
    error_message: Option<String>,
}

impl IntoResponse for ChangePasswordPageResponse {
    fn into_response(self) -> Response {
        let Self {
            return_to,
            user_name,
            error_message,
        } = self;

        let page = ChangePasswordPage {
            return_to,
            user_name,
            error_message,
        };

        render_html(&page, StatusCode::OK).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn render_change_password(user_name: &str, return_to: &str) -> String {
        let response = ChangePasswordPageResponse {
            return_to: return_to.to_string(),
            user_name: user_name.to_string(),
            error_message: None,
        }
        .into_response();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn change_password_posts_to_the_password_url_carrying_return_to() {
        let body = render_change_password("alice", "/authorize?client_id=1").await;

        assert!(
            body.contains("action=\"/login/password?return_to=%2Fauthorize%3Fclient_id%3D1\""),
            "{body}"
        );
    }

    #[tokio::test]
    async fn change_password_carries_the_user_name_escaped() {
        let body = render_change_password("\"><script>", "/authorize").await;

        assert!(
            body.contains(
                "name=\"user_name\" autocomplete=\"username\" value=\"&#34;&#62;&#60;script&#62;\""
            ),
            "{body}"
        );
    }

    async fn render(user_name: Option<&str>) -> String {
        render_with_return_to(user_name, "/authorize").await
    }

    async fn render_with_return_to(user_name: Option<&str>, return_to: &str) -> String {
        let response = LoginPageResponse {
            return_to: return_to.to_string(),
            user_name: user_name.map(str::to_string),
            error_message: None,
        }
        .into_response();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn posts_back_to_the_url_carrying_return_to() {
        let body =
            render_with_return_to(None, "/authorize?client_id=1&request_uri=urn%3Aabc").await;

        assert!(
            body.contains(
                "action=\"/login?return_to=%2Fauthorize%3Fclient_id%3D1%26request_uri%3Durn%253Aabc\""
            ),
            "{body}"
        );
    }

    #[tokio::test]
    async fn prefills_the_user_name_escaped() {
        let body = render(Some("\"><script>")).await;

        assert!(
            body.contains("value=\"&#34;&#62;&#60;script&#62;\""),
            "{body}"
        );
    }

    #[tokio::test]
    async fn leaves_the_user_name_empty_without_one() {
        let body = render(None).await;

        assert!(!body.contains("name=\"user_name\" autocomplete=\"username\" value="));
    }
}
