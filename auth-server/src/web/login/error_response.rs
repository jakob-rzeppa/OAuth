use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

pub enum LoginErrorResponse {
    MalformedRequest,
    InvalidReturnTo,

    MissingSessionCookie,
    InvalidCsrfToken,

    DatabaseError,
}

#[derive(Template)]
#[template(path = "error.html")]
struct LoginErrorPage {
    error: String,
    error_description: String,
}

impl IntoResponse for LoginErrorResponse {
    fn into_response(self) -> Response {
        let (status_code, error, error_description) = match self {
            LoginErrorResponse::MalformedRequest => (
                StatusCode::BAD_REQUEST,
                "invalid_request".to_string(),
                "The request is malformed.".to_string(),
            ),
            LoginErrorResponse::MissingSessionCookie => (
                StatusCode::BAD_REQUEST,
                "invalid_request".to_string(),
                "The login session cookie is missing. Reload the login page and try again."
                    .to_string(),
            ),
            LoginErrorResponse::InvalidCsrfToken => (
                StatusCode::BAD_REQUEST,
                "invalid_request".to_string(),
                "The CSRF token is invalid. Reload the login page and try again.".to_string(),
            ),
            LoginErrorResponse::InvalidReturnTo => (
                StatusCode::BAD_REQUEST,
                "invalid_request".to_string(),
                "The return_to parameter is missing or is not a path on this server.".to_string(),
            ),
            LoginErrorResponse::DatabaseError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "server_error".to_string(),
                "An unexpected error occurred while processing the request.".to_string(),
            ),
        };

        let page = LoginErrorPage {
            error,
            error_description,
        };

        match page.render() {
            Ok(html) => (status_code, Html(html)).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
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
