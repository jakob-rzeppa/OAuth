use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use url::Url;

use crate::{
    config::iss,
    domain::entity::authorization_code::request::validate::{
        FatalValidationError, RedirectableValidationError, ValidationError,
    },
    persistence::pars::take::TakeParError,
    security::require_session::{LoginRedirect, RequireUserSessionError},
};

pub enum AuthorizeSubmitRedirectErrorResponse {
    InvalidCodeChallenge,
    UnsupportedResponseType,
    InvalidScope,
    ServerError,
    AccessDenied,
}

pub enum AuthorizeSubmitPageErrorResponse {
    MalformedRequest,
    ClientNotFound,
    ClientIdMismatch,
    InvalidRedirectUri,
    InvalidState,
    RequestNotFound,
    ServerError,
}

pub enum AuthorizeSubmitErrorResponse {
    Redirect {
        error: AuthorizeSubmitRedirectErrorResponse,
        redirect_uri: String,
        state: String,
    },
    Page {
        error: AuthorizeSubmitPageErrorResponse,
    },
    /// The user has no valid session, so they are sent to the login and back to the authorization afterwards.
    LoginRequired(LoginRedirect),
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

impl From<AuthorizeSubmitPageErrorResponse> for AuthorizeSubmitErrorResponse {
    fn from(error: AuthorizeSubmitPageErrorResponse) -> Self {
        AuthorizeSubmitErrorResponse::Page { error }
    }
}

impl From<RequireUserSessionError> for AuthorizeSubmitErrorResponse {
    fn from(error: RequireUserSessionError) -> Self {
        match error {
            RequireUserSessionError::LoginRequired(redirect) => {
                AuthorizeSubmitErrorResponse::LoginRequired(redirect)
            }
            RequireUserSessionError::ServerError => {
                AuthorizeSubmitPageErrorResponse::ServerError.into()
            }
        }
    }
}

impl From<TakeParError> for AuthorizeSubmitErrorResponse {
    fn from(_: TakeParError) -> Self {
        AuthorizeSubmitPageErrorResponse::ServerError.into()
    }
}

impl From<ValidationError> for AuthorizeSubmitErrorResponse {
    fn from(error: ValidationError) -> Self {
        match error {
            ValidationError::Fatal { error } => AuthorizeSubmitErrorResponse::Page {
                error: match error {
                    FatalValidationError::ClientIdMismatch => {
                        AuthorizeSubmitPageErrorResponse::ClientIdMismatch
                    }
                    FatalValidationError::InvalidRedirectUri => {
                        AuthorizeSubmitPageErrorResponse::InvalidRedirectUri
                    }
                    FatalValidationError::InvalidState => {
                        AuthorizeSubmitPageErrorResponse::InvalidState
                    }
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

#[derive(Template)]
#[template(path = "error.html")]
struct AuthorizeSubmitErrorPage {
    error: String,
    error_description: String,
}

impl IntoResponse for AuthorizeSubmitPageErrorResponse {
    fn into_response(self) -> Response {
        let (status_code, error, error_description) = match self {
            AuthorizeSubmitPageErrorResponse::MalformedRequest => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The request is malformed.",
            ),
            AuthorizeSubmitPageErrorResponse::ClientNotFound => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The client_id parameter does not match any registered client.",
            ),
            AuthorizeSubmitPageErrorResponse::ClientIdMismatch => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The client_id parameter does not match the client_id of the authorization request.",
            ),
            AuthorizeSubmitPageErrorResponse::InvalidRedirectUri => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The redirect_uri of the authorization request is not registered for the client.",
            ),
            AuthorizeSubmitPageErrorResponse::InvalidState => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The state of the authorization request is invalid.",
            ),
            AuthorizeSubmitPageErrorResponse::RequestNotFound => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The request_uri parameter does not match a pending authorization request, or it has expired.",
            ),
            AuthorizeSubmitPageErrorResponse::ServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "server_error",
                "An unexpected error occurred while processing the request.",
            ),
        };

        let page = AuthorizeSubmitErrorPage {
            error: error.to_string(),
            error_description: error_description.to_string(),
        };

        match page.render() {
            Ok(html) => (status_code, Html(html)).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
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
                    return AuthorizeSubmitPageErrorResponse::ServerError.into_response();
                };
                url.query_pairs_mut()
                    .append_pair("error", error.code())
                    .append_pair("error_description", error.description())
                    .append_pair("state", &state)
                    .append_pair("iss", iss());
                Redirect::to(&url.to_string()).into_response()
            }
            AuthorizeSubmitErrorResponse::LoginRequired(redirect) => redirect.into_response(),
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
                error: AuthorizeSubmitPageErrorResponse::ServerError
            }
        ));
        assert!(matches!(
            AuthorizeSubmitErrorResponse::from(RequireUserSessionError::ServerError),
            AuthorizeSubmitErrorResponse::Page {
                error: AuthorizeSubmitPageErrorResponse::ServerError
            }
        ));
    }
}
