//! The error codes of OAuth 2.0 (RFC 6749 §4.1.2.1 and §5.2) and the HTTP status each one
//! is answered with, so an endpoint can't pair a code with the wrong status.

use axum::http::StatusCode;

use crate::domain::entity::authorization_code::request::validate::{
    FatalValidationError, RedirectableValidationError, ValidationError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthErrorCode {
    InvalidRequest,
    InvalidClient,
    InvalidGrant,
    UnsupportedGrantType,
    UnsupportedResponseType,
    InvalidScope,
    AccessDenied,
    ServerError,
}

impl OAuthErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            OAuthErrorCode::InvalidRequest => "invalid_request",
            OAuthErrorCode::InvalidClient => "invalid_client",
            OAuthErrorCode::InvalidGrant => "invalid_grant",
            OAuthErrorCode::UnsupportedGrantType => "unsupported_grant_type",
            OAuthErrorCode::UnsupportedResponseType => "unsupported_response_type",
            OAuthErrorCode::InvalidScope => "invalid_scope",
            OAuthErrorCode::AccessDenied => "access_denied",
            OAuthErrorCode::ServerError => "server_error",
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            OAuthErrorCode::InvalidClient => StatusCode::UNAUTHORIZED,
            OAuthErrorCode::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}

impl ValidationError {
    /// The OAuth error code a client is told for this validation error.
    pub fn oauth_code(&self) -> OAuthErrorCode {
        match self {
            ValidationError::Fatal { .. } => OAuthErrorCode::InvalidRequest,
            ValidationError::Redirectable { error, .. } => match error {
                RedirectableValidationError::InvalidResponseType => {
                    OAuthErrorCode::UnsupportedResponseType
                }
                RedirectableValidationError::InvalidScope => OAuthErrorCode::InvalidScope,
                RedirectableValidationError::InvalidCodeChallengeMethod
                | RedirectableValidationError::InvalidCodeChallenge => {
                    OAuthErrorCode::InvalidRequest
                }
            },
        }
    }

    /// The error description a client is told for this validation error.
    pub fn description(&self) -> &'static str {
        match self {
            ValidationError::Fatal { error } => match error {
                FatalValidationError::ClientIdMismatch => {
                    "The client_id parameter does not match the client_id of the authorization request."
                }
                FatalValidationError::InvalidRedirectUri => {
                    "The redirect_uri parameter is not registered for the client."
                }
                FatalValidationError::InvalidState => "The state parameter is missing or invalid.",
            },
            ValidationError::Redirectable { error, .. } => match error {
                RedirectableValidationError::InvalidResponseType => {
                    "The response_type parameter is missing or invalid."
                }
                RedirectableValidationError::InvalidScope => {
                    "The scope parameter is missing or invalid."
                }
                RedirectableValidationError::InvalidCodeChallengeMethod => {
                    "The code_challenge_method parameter is missing or invalid."
                }
                RedirectableValidationError::InvalidCodeChallenge => {
                    "The code_challenge parameter is missing or invalid."
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_client_is_unauthorized_and_server_error_is_internal() {
        assert_eq!(
            OAuthErrorCode::InvalidClient.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            OAuthErrorCode::ServerError.status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            OAuthErrorCode::InvalidGrant.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn redirectable_validation_errors_map_to_their_rfc_6749_codes() {
        let redirectable =
            |error| ValidationError::redirectable(error, String::new(), String::new());

        assert_eq!(
            redirectable(RedirectableValidationError::InvalidResponseType).oauth_code(),
            OAuthErrorCode::UnsupportedResponseType
        );
        assert_eq!(
            redirectable(RedirectableValidationError::InvalidScope).oauth_code(),
            OAuthErrorCode::InvalidScope
        );
        assert_eq!(
            redirectable(RedirectableValidationError::InvalidCodeChallenge).oauth_code(),
            OAuthErrorCode::InvalidRequest
        );
    }
}
