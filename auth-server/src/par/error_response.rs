use axum::extract::rejection::FormRejection;

use crate::{
    domain::authorization_code::request::validate::{FatalValidationError, ValidationError},
    error::{InternalError, api::ApiError, into_response_via},
    util::oauth_error::OAuthErrorCode,
};

#[derive(Debug)]
pub enum ParErrorResponse {
    InvalidRequestBody,
    InvalidClientId,
    ClientNotFound,
    /// The pushed request failed validation against the client.
    InvalidAuthorizationRequest {
        code: OAuthErrorCode,
        description: &'static str,
    },
    ServerError(InternalError),
}

impl From<ParErrorResponse> for ApiError {
    fn from(error: ParErrorResponse) -> Self {
        use ParErrorResponse::*;
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
            InvalidAuthorizationRequest { code, description } => ApiError::new(code, description),
            ServerError(source) => ApiError::Server(source),
        }
    }
}

into_response_via!(ParErrorResponse => ApiError);

impl From<InternalError> for ParErrorResponse {
    fn from(error: InternalError) -> Self {
        ParErrorResponse::ServerError(error)
    }
}

impl From<FormRejection> for ParErrorResponse {
    fn from(_: FormRejection) -> Self {
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
