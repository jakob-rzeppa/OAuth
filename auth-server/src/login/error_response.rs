use api_macros::ApiErrorResponse;
use axum::extract::rejection::FormRejection;

use crate::{
    error::InternalError,
    util::{html::render_error_page, oauth_error::OAuthErrorCode},
};

#[ApiErrorResponse(render = render_error_page)]
pub enum LoginErrorResponse {
    #[code(OAuthErrorCode::InvalidRequest)]
    #[description("The request is malformed.")]
    MalformedRequest,

    #[code(OAuthErrorCode::InvalidRequest)]
    #[description("The return_to parameter is missing or is not a path on this server.")]
    InvalidReturnTo,

    #[code(OAuthErrorCode::InvalidRequest)]
    #[description("The login session cookie is missing. Reload the login page and try again.")]
    MissingSessionCookie,

    #[code(OAuthErrorCode::InvalidRequest)]
    #[description("The CSRF token is invalid. Reload the login page and try again.")]
    InvalidCsrfToken,

    #[server_error]
    ServerError(InternalError),
}

impl From<FormRejection> for LoginErrorResponse {
    fn from(_: FormRejection) -> Self {
        LoginErrorResponse::MalformedRequest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, response::IntoResponse};

    #[tokio::test]
    async fn server_error_renders_server_error_page() {
        let response = LoginErrorResponse::ServerError(InternalError::Invariant("redis is down"))
            .into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("<h1>server_error</h1>"));
        assert!(!body.contains("redis is down"), "{body}");
    }
}
