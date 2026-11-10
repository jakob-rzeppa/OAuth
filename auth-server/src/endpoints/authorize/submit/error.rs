use axum::response::{IntoResponse, Response};

use crate::{
    endpoints::authorize::error::AuthorizeError,
    domain::authorization_code::request::validate::ValidationError,
    error::{InternalError, redirect::RedirectError},
    session::require_login::RequireLoginError,
};

pub enum AuthorizeSubmitError {
    /// Reported back to the client. Only used once the client and its `redirect_uri` are known
    /// to be valid.
    Redirect(RedirectError),
    /// Rendered as an error page, or, when the user has no valid session, sent to the login
    /// and back to the authorization afterwards.
    Page { error: AuthorizeError },
}

impl From<RedirectError> for AuthorizeSubmitError {
    fn from(error: RedirectError) -> Self {
        AuthorizeSubmitError::Redirect(error)
    }
}

impl From<AuthorizeError> for AuthorizeSubmitError {
    fn from(error: AuthorizeError) -> Self {
        AuthorizeSubmitError::Page { error }
    }
}

impl From<RequireLoginError> for AuthorizeSubmitError {
    fn from(error: RequireLoginError) -> Self {
        AuthorizeError::from(error).into()
    }
}

impl From<InternalError> for AuthorizeSubmitError {
    fn from(error: InternalError) -> Self {
        AuthorizeError::ServerError(error).into()
    }
}

impl From<ValidationError> for AuthorizeSubmitError {
    fn from(error: ValidationError) -> Self {
        let (code, description) = (error.oauth_code(), error.description());
        match error {
            ValidationError::Fatal { .. } => {
                AuthorizeError::InvalidAuthorizationRequest { code, description }.into()
            }
            ValidationError::Redirectable {
                redirect_uri,
                state,
                ..
            } => RedirectError::new(code, description, redirect_uri, state).into(),
        }
    }
}

impl IntoResponse for AuthorizeSubmitError {
    fn into_response(self) -> Response {
        match self {
            AuthorizeSubmitError::Page { error } => error.into_response(),
            AuthorizeSubmitError::Redirect(error) => error.into_response(),
        }
    }
}
