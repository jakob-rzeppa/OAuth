use api_macros::ApiErrorResponse;
use axum::extract::rejection::JsonRejection;

use crate::{
    domain::entity::authorization_code::request::validate::{
        FatalValidationError, ValidationError,
    },
    error::InternalError,
    util::oauth_error::OAuthErrorCode,
};

#[ApiErrorResponse]
pub enum ParErrorResponse {
    #[code(OAuthErrorCode::InvalidRequest)]
    #[description("Invalid request body.")]
    InvalidRequestBody,

    #[code(OAuthErrorCode::InvalidRequest)]
    #[description("The client_id parameter is missing or invalid.")]
    InvalidClientId,

    #[code(OAuthErrorCode::InvalidClient)]
    #[description("The client was not found.")]
    ClientNotFound,

    /// The pushed request failed validation against the client.
    #[code(code)]
    #[description("{description}")]
    InvalidAuthorizationRequest {
        code: OAuthErrorCode,
        description: &'static str,
    },

    #[server_error]
    ServerError(InternalError),
}

impl From<JsonRejection> for ParErrorResponse {
    fn from(_: JsonRejection) -> Self {
        ParErrorResponse::InvalidRequestBody
    }
}

impl From<ValidationError> for ParErrorResponse {
    fn from(error: ValidationError) -> Self {
        match error {
            // The request is built from the client it is validated against.
            ValidationError::Fatal {
                error: FatalValidationError::ClientIdMismatch,
            } => InternalError::Invariant(
                "a pushed authorization request was validated against another client",
            )
            .into(),
            error => ParErrorResponse::InvalidAuthorizationRequest {
                code: error.oauth_code(),
                description: error.description(),
            },
        }
    }
}
