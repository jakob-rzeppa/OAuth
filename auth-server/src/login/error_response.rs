use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::util::html::render_error_page;

pub enum LoginErrorResponse {
    MalformedRequest,
    InvalidReturnTo,

    MissingSessionCookie,
    InvalidCsrfToken,

    DatabaseError,
}

impl IntoResponse for LoginErrorResponse {
    fn into_response(self) -> Response {
        let (status_code, error, error_description) = match self {
            LoginErrorResponse::MalformedRequest => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The request is malformed.",
            ),
            LoginErrorResponse::MissingSessionCookie => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The login session cookie is missing. Reload the login page and try again.",
            ),
            LoginErrorResponse::InvalidCsrfToken => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The CSRF token is invalid. Reload the login page and try again.",
            ),
            LoginErrorResponse::InvalidReturnTo => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The return_to parameter is missing or is not a path on this server.",
            ),
            LoginErrorResponse::DatabaseError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "server_error",
                "An unexpected error occurred while processing the request.",
            ),
        };

        render_error_page(status_code, error, error_description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn database_error_renders_server_error_page() {
        let response = LoginErrorResponse::DatabaseError.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("<h1>server_error</h1>"));
    }
}
