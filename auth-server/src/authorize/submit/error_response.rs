use axum::response::{IntoResponse, Redirect, Response};
use url::Url;

use crate::{
    authorize::error_page::AuthorizeErrorPage, config::Config,
    domain::entity::authorization_code::request::validate::ValidationError, error::InternalError,
    security::require_session::RequireUserSessionError, util::oauth_error::OAuthErrorCode,
};

/// An error reported back to the client by redirecting to its `redirect_uri`
/// (RFC 6749 §4.1.2.1). Only used once the client and its `redirect_uri` are known to be valid.
pub struct ClientRedirectError {
    pub code: OAuthErrorCode,
    pub description: &'static str,
    pub redirect_uri: String,
    pub state: String,
    /// The failure behind a `server_error`, logged when the response is built.
    pub source: Option<InternalError>,
}

impl ClientRedirectError {
    pub fn access_denied(redirect_uri: String, state: String) -> Self {
        ClientRedirectError {
            code: OAuthErrorCode::AccessDenied,
            description: "The user denied the request.",
            redirect_uri,
            state,
            source: None,
        }
    }

    pub fn server_error(source: InternalError, redirect_uri: String, state: String) -> Self {
        ClientRedirectError {
            code: OAuthErrorCode::ServerError,
            description: "An unexpected error occurred while processing the request.",
            redirect_uri,
            state,
            source: Some(source),
        }
    }
}

impl IntoResponse for ClientRedirectError {
    fn into_response(self) -> Response {
        if let Some(source) = &self.source {
            tracing::error!(error = ?source, "request failed with a server error");
        }

        let Ok(mut url) = Url::parse(&self.redirect_uri) else {
            return AuthorizeErrorPage::ServerError(InternalError::Invariant(
                "a validated redirect_uri is not a valid URL",
            ))
            .into_response();
        };
        url.query_pairs_mut()
            .append_pair("error", self.code.as_str())
            .append_pair("error_description", self.description)
            .append_pair("state", &self.state)
            .append_pair("iss", Config::iss());
        Redirect::to(url.as_str()).into_response()
    }
}

pub enum AuthorizeSubmitErrorResponse {
    Redirect(ClientRedirectError),
    /// Rendered as an error page, or, when the user has no valid session, sent to the login
    /// and back to the authorization afterwards.
    Page {
        error: AuthorizeErrorPage,
    },
}

impl From<ClientRedirectError> for AuthorizeSubmitErrorResponse {
    fn from(error: ClientRedirectError) -> Self {
        AuthorizeSubmitErrorResponse::Redirect(error)
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
            } => ClientRedirectError {
                code,
                description,
                redirect_uri,
                state,
                source: None,
            }
            .into(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{StatusCode, header};

    #[test]
    fn internal_errors_become_page_server_errors() {
        assert!(matches!(
            AuthorizeSubmitErrorResponse::from(InternalError::Invariant("redis is down")),
            AuthorizeSubmitErrorResponse::Page {
                error: AuthorizeErrorPage::ServerError(_)
            }
        ));
        assert!(matches!(
            AuthorizeSubmitErrorResponse::from(RequireUserSessionError::Internal(
                InternalError::Invariant("redis is down")
            )),
            AuthorizeSubmitErrorResponse::Page {
                error: AuthorizeErrorPage::ServerError(_)
            }
        ));
    }

    #[test]
    fn redirect_errors_carry_the_code_and_state_to_the_client() {
        Config::iss_fake().setup(|| "https://issuer.example");

        let response = AuthorizeSubmitErrorResponse::from(ClientRedirectError::access_denied(
            "https://example.com/callback".to_string(),
            "the-state".to_string(),
        ))
        .into_response();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let location = Url::parse(response.headers()[header::LOCATION].to_str().unwrap()).unwrap();
        let query: Vec<(String, String)> = location.query_pairs().into_owned().collect();
        assert!(query.contains(&("error".to_string(), "access_denied".to_string())));
        assert!(query.contains(&("state".to_string(), "the-state".to_string())));
        assert!(query.contains(&("iss".to_string(), "https://issuer.example".to_string())));
    }
}
