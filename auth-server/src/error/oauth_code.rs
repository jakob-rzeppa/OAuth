//! The error codes of OAuth 2.0 (RFC 6749 §4.1.2.1 and §5.2), of protected resources
//! (RFC 6750 §3.1) and of request URIs (RFC 9101 §7), and the HTTP status each one is answered
//! with, so an endpoint can't pair a code with the wrong status.

use axum::http::StatusCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthErrorCode {
    InvalidRequest,
    InvalidClient,
    InvalidGrant,
    UnsupportedGrantType,
    UnsupportedResponseType,
    InvalidScope,
    AccessDenied,
    // Reported once an unknown or expired request_uri is answered with it.
    #[allow(dead_code)]
    InvalidRequestUri,
    InvalidToken,
    InsufficientScope,
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
            OAuthErrorCode::InvalidRequestUri => "invalid_request_uri",
            OAuthErrorCode::InvalidToken => "invalid_token",
            OAuthErrorCode::InsufficientScope => "insufficient_scope",
            OAuthErrorCode::ServerError => "server_error",
        }
    }

    /// `invalid_client` is a 400 as well: RFC 6749, section 5.2 only requires a 401 for clients
    /// that authenticated through the `Authorization` header, and a 401 has to carry a
    /// `WWW-Authenticate` challenge. All clients are public, so there is no scheme to name.
    pub fn status(&self) -> StatusCode {
        match self {
            OAuthErrorCode::InvalidToken => StatusCode::UNAUTHORIZED,
            OAuthErrorCode::InsufficientScope => StatusCode::FORBIDDEN,
            OAuthErrorCode::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_error_is_internal_and_everything_else_a_bad_request() {
        assert_eq!(
            OAuthErrorCode::InvalidClient.status(),
            StatusCode::BAD_REQUEST
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
    fn protected_resource_errors_are_unauthorized_and_forbidden() {
        assert_eq!(
            OAuthErrorCode::InvalidToken.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            OAuthErrorCode::InsufficientScope.status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            OAuthErrorCode::InvalidRequestUri.status(),
            StatusCode::BAD_REQUEST
        );
    }
}
