use axum::extract::rejection::FormRejection;

use crate::{
    error::{InternalError, api::ApiError, into_response_via},
    util::oauth_error::OAuthErrorCode,
};

#[derive(Debug)]
pub enum TokenErrorResponse {
    InvalidRequestBody,
    InvalidClientId,
    ClientNotFound,
    UnsupportedGrantType,
    MissingAuthorizationCode,
    MissingCodeVerifier,
    InvalidAuthorizationCode,
    InvalidCodeVerifier,
    ServerError(InternalError),
}

impl From<TokenErrorResponse> for ApiError {
    fn from(error: TokenErrorResponse) -> Self {
        use TokenErrorResponse::*;
        match error {
            InvalidRequestBody => ApiError::new(
                OAuthErrorCode::InvalidRequest,
                "Invalid request body.",
            ),
            InvalidClientId => ApiError::new(
                OAuthErrorCode::InvalidRequest,
                "The client_id parameter is missing or invalid.",
            ),
            ClientNotFound => ApiError::new(
                OAuthErrorCode::InvalidClient,
                "The client was not found.",
            ),
            UnsupportedGrantType => ApiError::new(
                OAuthErrorCode::UnsupportedGrantType,
                "Unsupported grant type.",
            ),
            MissingAuthorizationCode => ApiError::new(
                OAuthErrorCode::InvalidGrant,
                "Missing authorization code.",
            ),
            MissingCodeVerifier => ApiError::new(
                OAuthErrorCode::InvalidGrant,
                "Missing code verifier.",
            ),
            InvalidAuthorizationCode => ApiError::new(
                OAuthErrorCode::InvalidGrant,
                "Invalid or expired authorization code.",
            ),
            InvalidCodeVerifier => ApiError::new(
                OAuthErrorCode::InvalidGrant,
                "Invalid code verifier.",
            ),
            ServerError(source) => ApiError::Server(source),
        }
    }
}

into_response_via!(TokenErrorResponse => ApiError);

impl From<InternalError> for TokenErrorResponse {
    fn from(error: InternalError) -> Self {
        TokenErrorResponse::ServerError(error)
    }
}

impl From<FormRejection> for TokenErrorResponse {
    fn from(_: FormRejection) -> Self {
        TokenErrorResponse::InvalidRequestBody
    }
}
