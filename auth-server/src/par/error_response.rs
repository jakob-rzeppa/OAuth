use api_macros::ApiErrorResponse;
use axum::extract::rejection::JsonRejection;

#[ApiErrorResponse]
pub enum AuthorizePushErrorResponse {
    #[status_code(axum::http::StatusCode::BAD_REQUEST)]
    #[error("invalid_request")]
    #[description("Invalid request body.")]
    InvalidRequestBody,

    #[status_code(axum::http::StatusCode::BAD_REQUEST)]
    #[error("invalid_request")]
    #[description("The client_id parameter is missing or invalid.")]
    InvalidClientId,

    #[status_code(axum::http::StatusCode::UNAUTHORIZED)]
    #[error("invalid_client")]
    #[description("The client was not found.")]
    ClientNotFound,

    #[status_code(axum::http::StatusCode::BAD_REQUEST)]
    #[error("invalid_request")]
    #[description("The redirect_uri parameter is missing or invalid.")]
    InvalidRedirectUri,

    #[status_code(axum::http::StatusCode::BAD_REQUEST)]
    #[error("unsupported_response_type")]
    #[description("The response_type parameter is missing or invalid.")]
    InvalidResponseType,

    #[status_code(axum::http::StatusCode::BAD_REQUEST)]
    #[error("invalid_scope")]
    #[description("The scope parameter is missing or invalid.")]
    InvalidScope,

    #[status_code(axum::http::StatusCode::BAD_REQUEST)]
    #[error("invalid_request")]
    #[description("The state parameter is missing or invalid.")]
    InvalidState,

    #[status_code(axum::http::StatusCode::BAD_REQUEST)]
    #[error("invalid_request")]
    #[description("The code_challenge_method parameter is missing or invalid.")]
    InvalidCodeChallengeMethod,

    #[status_code(axum::http::StatusCode::BAD_REQUEST)]
    #[error("invalid_request")]
    #[description("The code_challenge parameter is missing or invalid.")]
    InvalidCodeChallenge,

    #[status_code(axum::http::StatusCode::INTERNAL_SERVER_ERROR)]
    #[error("server_error")]
    #[description("A database error occurred.")]
    DatabaseError,

    #[status_code(axum::http::StatusCode::INTERNAL_SERVER_ERROR)]
    #[error("server_error")]
    #[description("An internal server error occurred.")]
    InternalServerError,
}

impl From<JsonRejection> for AuthorizePushErrorResponse {
    fn from(_: JsonRejection) -> Self {
        AuthorizePushErrorResponse::InvalidRequestBody
    }
}
