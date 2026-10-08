use askama::Template;
use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};

use crate::{
    config::Config,
    domain::entity::login_session::LoginSession,
    login::{LOGIN_SESSION_COOKIE, error_response::LoginErrorResponse},
    persistence::login_session::save::save_login_session,
    util::{
        cookie::session_cookie,
        csrf::create_csrf_token,
        html::render_html,
        token::{hash_token, random_token},
    },
};

/// Starts a login session and builds the login form for it.
/// `user_name` pre-fills the user name field, e.g. after a failed login attempt.
pub(super) async fn render_login_page(
    return_to: String,
    user_name: Option<String>,
    error_message: Option<String>,
) -> Result<LoginPageResponse, LoginErrorResponse> {
    let session_token = random_token();
    let csrf_token = create_csrf_token();
    let session_ttl_seconds = Config::login_session_ttl();

    save_login_session(
        &hash_token(&session_token),
        LoginSession::new(csrf_token.clone()),
        session_ttl_seconds,
    )
    .await?;

    Ok(LoginPageResponse {
        csrf_token,
        return_to,
        session_token,
        session_ttl_seconds,
        user_name,
        error_message,
    })
}

pub struct LoginPageResponse {
    pub csrf_token: String,
    pub return_to: String,

    pub session_token: String,
    pub session_ttl_seconds: u64,

    // Pre-filled username
    pub user_name: Option<String>,

    // On error
    pub error_message: Option<String>,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage {
    csrf_token: String,
    return_to: String,

    // Pre-filled username
    user_name: Option<String>,

    // On error
    error_message: Option<String>,
}

impl IntoResponse for LoginPageResponse {
    fn into_response(self) -> Response {
        let Self {
            csrf_token,
            return_to,
            session_token,
            session_ttl_seconds,
            user_name,
            error_message,
        } = self;

        let page = LoginPage {
            csrf_token,
            return_to,
            user_name,
            error_message,
        };

        let Some(cookie) =
            session_cookie(LOGIN_SESSION_COOKIE, &session_token, session_ttl_seconds)
        else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };

        (
            [(header::SET_COOKIE, cookie)],
            render_html(&page, StatusCode::OK),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn render(user_name: Option<&str>) -> String {
        render_with_return_to(user_name, "/authorize").await
    }

    async fn render_with_return_to(user_name: Option<&str>, return_to: &str) -> String {
        let response = LoginPageResponse {
            csrf_token: "csrf".to_string(),
            return_to: return_to.to_string(),
            session_token: "token".to_string(),
            session_ttl_seconds: 900,
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
    async fn posts_back_to_the_url_carrying_return_to_instead_of_a_hidden_field() {
        let body =
            render_with_return_to(None, "/authorize?client_id=1&request_uri=urn%3Aabc").await;

        assert!(
            body.contains(
                "action=\"/login?return_to=%2Fauthorize%3Fclient_id%3D1%26request_uri%3Durn%253Aabc\""
            ),
            "{body}"
        );
        assert!(!body.contains("name=\"return_to\""), "{body}");
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
