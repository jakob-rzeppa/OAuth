use axum::{
    Form,
    extract::{FromRequest, FromRequestParts},
    http::header,
};
use serde::Deserialize;

use crate::{
    login::{LOGIN_SESSION_COOKIE, error_response::LoginErrorResponse},
    util::cookie::cookie_value,
};

/// Read from the request body, so it has to be the last extractor of a handler.
#[derive(Deserialize, FromRequest)]
#[from_request(via(Form), rejection(LoginErrorResponse))]
pub struct LoginFormSubmitRequest {
    pub csrf_token: String,

    pub user_name: String,
    pub password: String,
}

/// The login session token, read from the login session cookie.
pub struct LoginSessionToken(pub String);

impl<S: Send + Sync> FromRequestParts<S> for LoginSessionToken {
    type Rejection = LoginErrorResponse;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find_map(|cookies| cookie_value(cookies, LOGIN_SESSION_COOKIE))
            .map(|token| LoginSessionToken(token.to_string()))
            .ok_or(LoginErrorResponse::MissingSessionCookie)
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request};

    use super::*;

    #[tokio::test]
    async fn extractor_reads_the_token_from_the_cookie_header() {
        let (mut parts, _) = Request::post("/login")
            .header(header::COOKIE, "login_session=token-123")
            .body(())
            .unwrap()
            .into_parts();

        let token = LoginSessionToken::from_request_parts(&mut parts, &()).await;

        assert_eq!(token.ok().unwrap().0, "token-123");
    }

    #[tokio::test]
    async fn extractor_fails_without_the_cookie() {
        let (mut parts, _) = Request::post("/login").body(()).unwrap().into_parts();

        let token = LoginSessionToken::from_request_parts(&mut parts, &()).await;

        assert!(matches!(
            token,
            Err(LoginErrorResponse::MissingSessionCookie)
        ));
    }

    fn form_request(content_type: &str, body: &str) -> Request<Body> {
        Request::post("/login")
            .header(header::CONTENT_TYPE, content_type)
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn form_is_read_from_the_urlencoded_body() {
        let request = form_request(
            "application/x-www-form-urlencoded",
            "csrf_token=c%2Bsrf&user_name=alice&password=p%26ss",
        );

        let form = LoginFormSubmitRequest::from_request(request, &()).await;

        let form = form.ok().expect("expected the form to parse");
        assert_eq!(form.csrf_token, "c+srf");
        assert_eq!(form.user_name, "alice");
        assert_eq!(form.password, "p&ss");
    }

    #[tokio::test]
    async fn form_fails_when_a_field_is_missing() {
        let request = form_request(
            "application/x-www-form-urlencoded",
            "csrf_token=c&user_name=alice",
        );

        let form = LoginFormSubmitRequest::from_request(request, &()).await;

        assert!(matches!(form, Err(LoginErrorResponse::MalformedRequest)));
    }

    #[tokio::test]
    async fn form_fails_when_it_is_not_form_encoded() {
        let request = form_request("application/json", "{}");

        let form = LoginFormSubmitRequest::from_request(request, &()).await;

        assert!(matches!(form, Err(LoginErrorResponse::MalformedRequest)));
    }
}
