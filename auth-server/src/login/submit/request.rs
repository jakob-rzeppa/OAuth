use axum::{Form, extract::FromRequest};
use serde::Deserialize;

use crate::login::error_response::LoginErrorResponse;

/// Read from the request body, so it has to be the last extractor of a handler.
#[derive(Deserialize, FromRequest)]
#[from_request(via(Form), rejection(LoginErrorResponse))]
pub struct LoginFormSubmitRequest {
    pub user_name: String,
    pub password: String,
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request, http::header};

    use super::*;

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
