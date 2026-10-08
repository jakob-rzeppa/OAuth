use axum::response::{IntoResponse, Redirect, Response};
use url::Url;

use crate::{
    authorize::error_page::AuthorizeErrorPage,
    config::Config,
    domain::entity::authorization_code::request::validate::{
        FatalValidationError, RedirectableValidationError, ValidationError,
    },
    persistence::pars::take::TakeParError,
    security::require_session::RequireUserSessionError,
};

pub enum AuthorizeSubmitRedirectErrorResponse {
    InvalidCodeChallenge,
    UnsupportedResponseType,
    InvalidScope,
    ServerError,
    AccessDenied,
}

pub enum AuthorizeSubmitErrorResponse {
    Redirect {
        error: AuthorizeSubmitRedirectErrorResponse,
        redirect_uri: String,
        state: String,
    },
    /// Rendered as an error page, or, when the user has no valid session, sent to the login
    /// and back to the authorization afterwards.
    Page { error: AuthorizeErrorPage },
}

impl AuthorizeSubmitErrorResponse {
    pub fn server_error_redirect(redirect_uri: String, state: String) -> Self {
        AuthorizeSubmitErrorResponse::Redirect {
            error: AuthorizeSubmitRedirectErrorResponse::ServerError,
            redirect_uri,
            state,
        }
    }
}

impl From<AuthorizeErrorPage> for AuthorizeSubmitErrorResponse {
    fn from(error: AuthorizeErrorPage) -> Self {
        AuthorizeSubmitErrorResponse::Page { error }
    }
}

impl From<RequireUserSessionError> for AuthorizeSubmitErrorResponse {
    fn from(error: RequireUserSessionError) -> Self {
        AuthorizeErrorPage::from(error).into()
    }
}

impl From<TakeParError> for AuthorizeSubmitErrorResponse {
    fn from(_: TakeParError) -> Self {
        AuthorizeErrorPage::ServerError.into()
    }
}

impl From<ValidationError> for AuthorizeSubmitErrorResponse {
    fn from(error: ValidationError) -> Self {
        match error {
            ValidationError::Fatal { error } => AuthorizeSubmitErrorResponse::Page {
                error: match error {
                    FatalValidationError::ClientIdMismatch => AuthorizeErrorPage::ClientIdMismatch,
                    FatalValidationError::InvalidRedirectUri => {
                        AuthorizeErrorPage::InvalidRedirectUri
                    }
                    FatalValidationError::InvalidState => AuthorizeErrorPage::InvalidState,
                },
            },
            ValidationError::Redirectable {
                error,
                redirect_uri,
                state,
            } => AuthorizeSubmitErrorResponse::Redirect {
                error: match error {
                    RedirectableValidationError::InvalidResponseType => {
                        AuthorizeSubmitRedirectErrorResponse::UnsupportedResponseType
                    }
                    RedirectableValidationError::InvalidScope => {
                        AuthorizeSubmitRedirectErrorResponse::InvalidScope
                    }
                    RedirectableValidationError::InvalidCodeChallengeMethod
                    | RedirectableValidationError::InvalidCodeChallenge => {
                        AuthorizeSubmitRedirectErrorResponse::InvalidCodeChallenge
                    }
                },
                redirect_uri,
                state,
            },
        }
    }
}

impl AuthorizeSubmitRedirectErrorResponse {
    fn code(&self) -> &'static str {
        match self {
            AuthorizeSubmitRedirectErrorResponse::InvalidCodeChallenge => "invalid_request",
            AuthorizeSubmitRedirectErrorResponse::UnsupportedResponseType => {
                "unsupported_response_type"
            }
            AuthorizeSubmitRedirectErrorResponse::InvalidScope => "invalid_scope",
            AuthorizeSubmitRedirectErrorResponse::ServerError => "server_error",
            AuthorizeSubmitRedirectErrorResponse::AccessDenied => "access_denied",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            AuthorizeSubmitRedirectErrorResponse::InvalidCodeChallenge => {
                "The code challenge is invalid."
            }
            AuthorizeSubmitRedirectErrorResponse::UnsupportedResponseType => {
                "The response type is unsupported."
            }
            AuthorizeSubmitRedirectErrorResponse::InvalidScope => "The scope is invalid.",
            AuthorizeSubmitRedirectErrorResponse::ServerError => "An unexpected error occurred.",
            AuthorizeSubmitRedirectErrorResponse::AccessDenied => "The user denied the request.",
        }
    }
}

impl IntoResponse for AuthorizeSubmitErrorResponse {
    fn into_response(self) -> Response {
        match self {
            AuthorizeSubmitErrorResponse::Page { error } => error.into_response(),
            AuthorizeSubmitErrorResponse::Redirect {
                error,
                redirect_uri,
                state,
            } => {
                let Ok(mut url) = Url::parse(&redirect_uri) else {
                    return AuthorizeErrorPage::ServerError.into_response();
                };
                url.query_pairs_mut()
                    .append_pair("error", error.code())
                    .append_pair("error_description", error.description())
                    .append_pair("state", &state)
                    .append_pair("iss", Config::iss());
                Redirect::to(&url.to_string()).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistence_errors_become_page_server_errors() {
        assert!(matches!(
            AuthorizeSubmitErrorResponse::from(TakeParError::DatabaseError),
            AuthorizeSubmitErrorResponse::Page {
                error: AuthorizeErrorPage::ServerError
            }
        ));
        assert!(matches!(
            AuthorizeSubmitErrorResponse::from(RequireUserSessionError::ServerError),
            AuthorizeSubmitErrorResponse::Page {
                error: AuthorizeErrorPage::ServerError
            }
        ));
    }
}
