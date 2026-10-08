use api_macros::ApiErrorResponse;
use axum::extract::rejection::JsonRejection;

use crate::{error::InternalError, util::oauth_error::OAuthErrorCode};

#[ApiErrorResponse]
pub enum TokenErrorResponse {
    #[code(OAuthErrorCode::InvalidRequest)]
    #[description("Invalid request body.")]
    InvalidRequestBody,

    #[code(OAuthErrorCode::InvalidRequest)]
    #[description("The client_id parameter is missing or invalid.")]
    InvalidClientId,

    #[code(OAuthErrorCode::InvalidClient)]
    #[description("The client was not found.")]
    ClientNotFound,

    #[code(OAuthErrorCode::UnsupportedGrantType)]
    #[description("Unsupported grant type.")]
    UnsupportedGrantType,

    #[code(OAuthErrorCode::InvalidGrant)]
    #[description("Missing authorization code.")]
    MissingAuthorizationCode,

    #[code(OAuthErrorCode::InvalidGrant)]
    #[description("Missing code verifier.")]
    MissingCodeVerifier,

    #[code(OAuthErrorCode::InvalidGrant)]
    #[description("Invalid or expired authorization code.")]
    InvalidAuthorizationCode,

    #[code(OAuthErrorCode::InvalidGrant)]
    #[description("Invalid code verifier.")]
    InvalidCodeVerifier,

    #[server_error]
    ServerError(InternalError),
}

impl From<JsonRejection> for TokenErrorResponse {
    fn from(_: JsonRejection) -> Self {
        TokenErrorResponse::InvalidRequestBody
    }
}
