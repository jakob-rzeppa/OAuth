use axum::extract::rejection::FormRejection;

use crate::error::{InternalError, api::ApiError, into_response_via, oauth_code::OAuthErrorCode};

#[derive(Debug)]
pub enum TokenError {
    InvalidRequestBody,
    InvalidClientId,
    ClientNotFound,
    UnsupportedGrantType,
    MissingAuthorizationCode,
    MissingCodeVerifier,
    MalformedCodeVerifier,
    InvalidAuthorizationCode,
    InvalidCodeVerifier,
    ServerError(InternalError),
}

impl From<TokenError> for ApiError {
    fn from(error: TokenError) -> Self {
        use TokenError::*;
        match error {
            InvalidRequestBody => {
                ApiError::new(OAuthErrorCode::InvalidRequest, "Invalid request body.")
            }
            InvalidClientId => ApiError::new(
                OAuthErrorCode::InvalidRequest,
                "The client_id parameter is missing or invalid.",
            ),
            ClientNotFound => {
                ApiError::new(OAuthErrorCode::InvalidClient, "The client was not found.")
            }
            UnsupportedGrantType => ApiError::new(
                OAuthErrorCode::UnsupportedGrantType,
                "Unsupported grant type.",
            ),
            MissingAuthorizationCode => ApiError::new(
                OAuthErrorCode::InvalidRequest,
                "Missing authorization code.",
            ),
            MissingCodeVerifier => {
                ApiError::new(OAuthErrorCode::InvalidRequest, "Missing code verifier.")
            }
            MalformedCodeVerifier => ApiError::new(
                OAuthErrorCode::InvalidRequest,
                "The code_verifier parameter is malformed.",
            ),
            InvalidAuthorizationCode => ApiError::new(
                OAuthErrorCode::InvalidGrant,
                "Invalid or expired authorization code.",
            ),
            InvalidCodeVerifier => {
                ApiError::new(OAuthErrorCode::InvalidGrant, "Invalid code verifier.")
            }
            ServerError(source) => ApiError::Server(source),
        }
    }
}

into_response_via!(TokenError => ApiError);

impl From<InternalError> for TokenError {
    fn from(error: InternalError) -> Self {
        TokenError::ServerError(error)
    }
}

impl From<FormRejection> for TokenError {
    fn from(_: FormRejection) -> Self {
        TokenError::InvalidRequestBody
    }
}
