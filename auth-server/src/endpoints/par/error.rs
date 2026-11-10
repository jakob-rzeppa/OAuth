use axum::extract::rejection::FormRejection;

use crate::{
    domain::authorization_code::request::validate::{FatalValidationError, ValidationError},
    error::{InternalError, api::ApiError, into_response_via, oauth_code::OAuthErrorCode},
};

#[derive(Debug)]
pub enum ParError {
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

impl From<ParError> for ApiError {
    fn from(error: ParError) -> Self {
        use ParError::*;
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
            InvalidAuthorizationRequest { code, description } => ApiError::new(code, description),
            ServerError(source) => ApiError::Server(source),
        }
    }
}

into_response_via!(ParError => ApiError);

impl From<InternalError> for ParError {
    fn from(error: InternalError) -> Self {
        ParError::ServerError(error)
    }
}

impl From<FormRejection> for ParError {
    fn from(_: FormRejection) -> Self {
        ParError::InvalidRequestBody
    }
}

impl From<ValidationError> for ParError {
    fn from(error: ValidationError) -> Self {
        match error {
            // The request is built from the client it is validated against.
            ValidationError::Fatal {
                error: FatalValidationError::ClientIdMismatch,
            } => InternalError::Invariant(
                "a pushed authorization request was validated against another client",
            )
            .into(),
            error => ParError::InvalidAuthorizationRequest {
                code: error.oauth_code(),
                description: error.description(),
            },
        }
    }
}
