use askama::Template;
use axum::{
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};

use crate::web::login::cookie::{LOGIN_SESSION_COOKIE, session_cookie};

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

        let html = match page.render() {
            Ok(html) => html,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };

        let Some(cookie) =
            session_cookie(LOGIN_SESSION_COOKIE, &session_token, session_ttl_seconds)
        else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };

        (StatusCode::OK, [(header::SET_COOKIE, cookie)], Html(html)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn render(user_name: Option<&str>) -> String {
        let response = LoginPageResponse {
            csrf_token: "csrf".to_string(),
            return_to: "/authorize".to_string(),
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
