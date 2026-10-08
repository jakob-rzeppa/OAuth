use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    security::require_session::{LoginRedirect, RequireUserSessionError},
    util::html::render_error_page,
};

/// Errors of the consent page and its submit that are not redirected back to the client:
/// they render the error page, or send the user to the login first.
pub enum AuthorizeErrorPage {
    MalformedRequest,

    MissingClientId,
    InvalidClientId,
    ClientNotFound,

    MissingRequestUri,
    RequestNotFound,

    ClientIdMismatch,
    InvalidRedirectUri,
    InvalidState,

    ServerError,

    LoginRequired(LoginRedirect),
}

impl From<RequireUserSessionError> for AuthorizeErrorPage {
    fn from(error: RequireUserSessionError) -> Self {
        match error {
            RequireUserSessionError::LoginRequired(redirect) => {
                AuthorizeErrorPage::LoginRequired(redirect)
            }
            RequireUserSessionError::ServerError => AuthorizeErrorPage::ServerError,
        }
    }
}

impl IntoResponse for AuthorizeErrorPage {
    fn into_response(self) -> Response {
        let (status_code, error, error_description) = match self {
            AuthorizeErrorPage::MalformedRequest => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The request is malformed.",
            ),
            AuthorizeErrorPage::MissingClientId => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The client_id parameter is missing.",
            ),
            AuthorizeErrorPage::InvalidClientId => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The client_id parameter is invalid.",
            ),
            AuthorizeErrorPage::ClientNotFound => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The client_id parameter does not match any registered client.",
            ),
            AuthorizeErrorPage::MissingRequestUri => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The request_uri parameter is missing.",
            ),
            AuthorizeErrorPage::RequestNotFound => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The request_uri parameter does not match a pending authorization request, or it has expired.",
            ),
            AuthorizeErrorPage::ClientIdMismatch => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The client_id parameter does not match the client_id of the authorization request.",
            ),
            AuthorizeErrorPage::InvalidRedirectUri => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The redirect_uri of the authorization request is not registered for the client.",
            ),
            AuthorizeErrorPage::InvalidState => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The state of the authorization request is invalid.",
            ),
            AuthorizeErrorPage::ServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "server_error",
                "An unexpected error occurred while processing the request.",
            ),
            AuthorizeErrorPage::LoginRequired(redirect) => {
                return redirect.into_response();
            }
        };

        render_error_page(status_code, error, error_description)
    }
}
