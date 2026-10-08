use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

use crate::security::require_session::{LoginRedirect, RequireUserSessionError};

pub enum AuthorizePageErrorResponse {
    MalformedRequest,

    MissingClientId,
    InvalidClientId,
    ClientNotFound,

    MissingRequestUri,
    RequestNotFound,

    ClientIdMismatch,

    ServerError,

    /// The user has no valid session, so they are sent to the login and back to the authorization afterwards.
    LoginRequired(LoginRedirect),
}

impl From<RequireUserSessionError> for AuthorizePageErrorResponse {
    fn from(error: RequireUserSessionError) -> Self {
        match error {
            RequireUserSessionError::LoginRequired(redirect) => {
                AuthorizePageErrorResponse::LoginRequired(redirect)
            }
            RequireUserSessionError::ServerError => AuthorizePageErrorResponse::ServerError,
        }
    }
}

#[derive(Template)]
#[template(path = "error.html")]
struct AuthorizePageErrorPage {
    error: String,
    error_description: String,
}

impl IntoResponse for AuthorizePageErrorResponse {
    fn into_response(self) -> Response {
        let (status_code, error, error_description) = match self {
            AuthorizePageErrorResponse::MalformedRequest => (
                StatusCode::BAD_REQUEST,
                "invalid_request".to_string(),
                "The request is malformed.".to_string(),
            ),
            AuthorizePageErrorResponse::MissingClientId => (
                StatusCode::BAD_REQUEST,
                "invalid_request".to_string(),
                "The client_id parameter is missing.".to_string(),
            ),
            AuthorizePageErrorResponse::InvalidClientId => (
                StatusCode::BAD_REQUEST,
                "invalid_request".to_string(),
                "The client_id parameter is invalid.".to_string(),
            ),
            AuthorizePageErrorResponse::ClientNotFound => (
                StatusCode::BAD_REQUEST,
                "invalid_request".to_string(),
                "The client_id parameter does not match any registered client.".to_string(),
            ),
            AuthorizePageErrorResponse::MissingRequestUri => (
                StatusCode::BAD_REQUEST,
                "invalid_request".to_string(),
                "The request_uri parameter is missing.".to_string(),
            ),
            AuthorizePageErrorResponse::RequestNotFound => (
                StatusCode::BAD_REQUEST,
                "invalid_request".to_string(),
                "The request_uri parameter does not match a pending authorization request, or it has expired.".to_string(),
            ),
            AuthorizePageErrorResponse::ClientIdMismatch => (
                StatusCode::BAD_REQUEST,
                "invalid_request".to_string(),
                "The client_id parameter does not match the client_id of the authorization request.".to_string(),
            ),
            AuthorizePageErrorResponse::ServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "server_error".to_string(),
                "An unexpected error occurred while processing the request.".to_string(),
            ),
            AuthorizePageErrorResponse::LoginRequired(redirect) => {
                return redirect.into_response();
            }
        };

        let page = AuthorizePageErrorPage {
            error,
            error_description,
        };

        match page.render() {
            Ok(html) => (status_code, Html(html)).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
