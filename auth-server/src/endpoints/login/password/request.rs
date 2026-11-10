use axum::{Form, extract::FromRequest};
use serde::Deserialize;

use crate::endpoints::login::error::LoginError;

/// Read from the request body, so it has to be the last extractor of a handler.
#[derive(Deserialize, FromRequest)]
#[from_request(via(Form), rejection(LoginError))]
pub struct ChangePasswordFormSubmitRequest {
    pub user_name: String,
    pub current_password: String,
    pub new_password: String,
    pub new_password_confirmation: String,
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request, http::header};

    use super::*;

    fn form_request(body: &str) -> Request<Body> {
        Request::post("/login/password")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn form_is_read_from_the_urlencoded_body() {
        let request = form_request(
            "user_name=alice&current_password=tmp&new_password=n%26w&new_password_confirmation=n%26w",
        );

        let form = ChangePasswordFormSubmitRequest::from_request(request, &()).await;

        let form = form.ok().expect("expected the form to parse");
        assert_eq!(form.user_name, "alice");
        assert_eq!(form.current_password, "tmp");
        assert_eq!(form.new_password, "n&w");
        assert_eq!(form.new_password_confirmation, "n&w");
    }

    #[tokio::test]
    async fn form_fails_when_a_field_is_missing() {
        let request = form_request("user_name=alice&current_password=tmp&new_password=new");

        let form = ChangePasswordFormSubmitRequest::from_request(request, &()).await;

        assert!(matches!(form, Err(LoginError::MalformedRequest)));
    }
}
