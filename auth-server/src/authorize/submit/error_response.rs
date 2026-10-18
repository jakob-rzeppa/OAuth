use axum::response::{IntoResponse, Response};

use crate::{
    authorize::error_page::AuthorizeErrorPage,
    domain::authorization_code::request::validate::ValidationError,
    error::{InternalError, redirect::RedirectError},
    security::require_login::RequireLoginError,
};

pub enum AuthorizeSubmitErrorResponse {
    /// Reported back to the client. Only used once the client and its `redirect_uri` are known
    /// to be valid.
    Redirect(RedirectError),
    /// Rendered as an error page, or, when the user has no valid session, sent to the login
    /// and back to the authorization afterwards.
    Page { error: AuthorizeErrorPage },
}

impl From<RedirectError> for AuthorizeSubmitErrorResponse {
    fn from(error: RedirectError) -> Self {
        AuthorizeSubmitErrorResponse::Redirect(error)
    }
}

impl From<AuthorizeErrorPage> for AuthorizeSubmitErrorResponse {
    fn from(error: AuthorizeErrorPage) -> Self {
        AuthorizeSubmitErrorResponse::Page { error }
    }
}

impl From<RequireLoginError> for AuthorizeSubmitErrorResponse {
    fn from(error: RequireLoginError) -> Self {
        AuthorizeErrorPage::from(error).into()
    }
}

impl From<InternalError> for AuthorizeSubmitErrorResponse {
    fn from(error: InternalError) -> Self {
        AuthorizeErrorPage::ServerError(error).into()
    }
}

impl From<ValidationError> for AuthorizeSubmitErrorResponse {
    fn from(error: ValidationError) -> Self {
        let (code, description) = (error.oauth_code(), error.description());
        match error {
            ValidationError::Fatal { .. } => {
                AuthorizeErrorPage::InvalidAuthorizationRequest { code, description }.into()
            }
            ValidationError::Redirectable {
                redirect_uri,
                state,
                ..
            } => RedirectError::new(code, description, redirect_uri, state).into(),
        }
    }
}

impl IntoResponse for AuthorizeSubmitErrorResponse {
    fn into_response(self) -> Response {
        match self {
            AuthorizeSubmitErrorResponse::Page { error } => error.into_response(),
            AuthorizeSubmitErrorResponse::Redirect(error) => error.into_response(),
        }
    }
}
