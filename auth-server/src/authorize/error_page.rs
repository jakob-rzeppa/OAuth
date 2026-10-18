use axum::{
    extract::rejection::{FormRejection, QueryRejection},
    response::{IntoResponse, Response},
};

use crate::{
    error::{InternalError, page::PageError},
    security::require_login::{LoginRedirect, RequireLoginError},
    util::oauth_error::OAuthErrorCode,
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
    /// The pushed request failed validation in a way that can't be redirected to the client.
    InvalidAuthorizationRequest {
        code: OAuthErrorCode,
        description: &'static str,
    },
    ServerError(InternalError),
    LoginRequired(LoginRedirect),
}

impl IntoResponse for AuthorizeErrorPage {
    fn into_response(self) -> Response {
        use AuthorizeErrorPage::*;
        let error = match self {
            LoginRequired(redirect) => return redirect.into_response(),
            MalformedRequest => PageError::new(
                OAuthErrorCode::InvalidRequest,
                "The request is malformed.",
            ),
            MissingClientId => PageError::new(
                OAuthErrorCode::InvalidRequest,
                "The client_id parameter is missing.",
            ),
            InvalidClientId => PageError::new(
                OAuthErrorCode::InvalidRequest,
                "The client_id parameter is invalid.",
            ),
            ClientNotFound => PageError::new(
                OAuthErrorCode::InvalidRequest,
                "The client_id parameter does not match any registered client.",
            ),
            MissingRequestUri => PageError::new(
                OAuthErrorCode::InvalidRequest,
                "The request_uri parameter is missing.",
            ),
            RequestNotFound => PageError::new(
                OAuthErrorCode::InvalidRequest,
                "The request_uri parameter does not match a pending authorization request, or it has expired.",
            ),
            ClientIdMismatch => PageError::new(
                OAuthErrorCode::InvalidRequest,
                "The client_id parameter does not match the client_id of the authorization request.",
            ),
            InvalidAuthorizationRequest { code, description } => PageError::new(code, description),
            ServerError(source) => PageError::Server(source),
        };
        error.into_response()
    }
}

impl From<InternalError> for AuthorizeErrorPage {
    fn from(error: InternalError) -> Self {
        AuthorizeErrorPage::ServerError(error)
    }
}

impl From<RequireLoginError> for AuthorizeErrorPage {
    fn from(error: RequireLoginError) -> Self {
        match error {
            RequireLoginError::LoginRequired(redirect) => {
                AuthorizeErrorPage::LoginRequired(redirect)
            }
            RequireLoginError::Internal(error) => AuthorizeErrorPage::ServerError(error),
        }
    }
}

impl From<QueryRejection> for AuthorizeErrorPage {
    fn from(_: QueryRejection) -> Self {
        AuthorizeErrorPage::MalformedRequest
    }
}

impl From<FormRejection> for AuthorizeErrorPage {
    fn from(_: FormRejection) -> Self {
        AuthorizeErrorPage::MalformedRequest
    }
}
