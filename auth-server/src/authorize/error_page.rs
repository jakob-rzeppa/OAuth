use api_macros::ApiErrorResponse;
use axum::extract::rejection::{FormRejection, QueryRejection};

use crate::{
    error::InternalError,
    security::require_session::{LoginRedirect, RequireUserSessionError},
    util::{html::render_error_page, oauth_error::OAuthErrorCode},
};

/// Errors of the consent page and its submit that are not redirected back to the client:
/// they render the error page, or send the user to the login first.
#[ApiErrorResponse(render = render_error_page)]
pub enum AuthorizeErrorPage {
    #[code(OAuthErrorCode::InvalidRequest)]
    #[description("The request is malformed.")]
    MalformedRequest,

    #[code(OAuthErrorCode::InvalidRequest)]
    #[description("The client_id parameter is missing.")]
    MissingClientId,

    #[code(OAuthErrorCode::InvalidRequest)]
    #[description("The client_id parameter is invalid.")]
    InvalidClientId,

    #[code(OAuthErrorCode::InvalidRequest)]
    #[description("The client_id parameter does not match any registered client.")]
    ClientNotFound,

    #[code(OAuthErrorCode::InvalidRequest)]
    #[description("The request_uri parameter is missing.")]
    MissingRequestUri,

    #[code(OAuthErrorCode::InvalidRequest)]
    #[description(
        "The request_uri parameter does not match a pending authorization request, or it has expired."
    )]
    RequestNotFound,

    #[code(OAuthErrorCode::InvalidRequest)]
    #[description(
        "The client_id parameter does not match the client_id of the authorization request."
    )]
    ClientIdMismatch,

    /// The pushed request failed validation in a way that can't be redirected to the client.
    #[code(code)]
    #[description("{description}")]
    InvalidAuthorizationRequest {
        code: OAuthErrorCode,
        description: &'static str,
    },

    #[server_error]
    ServerError(InternalError),

    #[into_response]
    LoginRequired(LoginRedirect),
}

impl From<RequireUserSessionError> for AuthorizeErrorPage {
    fn from(error: RequireUserSessionError) -> Self {
        match error {
            RequireUserSessionError::LoginRequired(redirect) => {
                AuthorizeErrorPage::LoginRequired(redirect)
            }
            RequireUserSessionError::Internal(error) => AuthorizeErrorPage::ServerError(error),
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
