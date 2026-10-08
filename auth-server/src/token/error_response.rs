use api_macros::ApiErrorResponse;
use axum::extract::rejection::JsonRejection;

#[ApiErrorResponse]
pub enum TokenErrorResponse {
    #[status_code(axum::http::StatusCode::BAD_REQUEST)]
    #[error("invalid_request")]
    #[description("Invalid request body.")]
    InvalidRequestBody,

    #[status_code(axum::http::StatusCode::BAD_REQUEST)]
    #[error("invalid_client")]
    #[description("Invalid client ID.")]
    InvalidClientId,

    #[status_code(axum::http::StatusCode::BAD_REQUEST)]
    #[error("unsupported_grant_type")]
    #[description("Unsupported grant type.")]
    UnsupportedGrantType,

    #[status_code(axum::http::StatusCode::BAD_REQUEST)]
    #[error("invalid_grant")]
    #[description("Missing authorization code.")]
    MissingAuthorizationCode,

    #[status_code(axum::http::StatusCode::BAD_REQUEST)]
    #[error("invalid_grant")]
    #[description("Missing code verifier.")]
    MissingCodeVerifier,

    #[status_code(axum::http::StatusCode::INTERNAL_SERVER_ERROR)]
    #[error("server_error")]
    #[description("Internal server error.")]
    DatabaseError,

    #[status_code(axum::http::StatusCode::BAD_REQUEST)]
    #[error("invalid_grant")]
    #[description("Invalid or expired authorization code.")]
    InvalidAuthorizationCode,

    #[status_code(axum::http::StatusCode::BAD_REQUEST)]
    #[error("invalid_grant")]
    #[description("Invalid code verifier.")]
    InvalidCodeVerifier,
}

impl From<JsonRejection> for TokenErrorResponse {
    fn from(_: JsonRejection) -> Self {
        TokenErrorResponse::InvalidRequestBody
    }
}
