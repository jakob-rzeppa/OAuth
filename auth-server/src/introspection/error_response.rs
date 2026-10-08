use api_macros::ApiErrorResponse;
use axum::http::StatusCode;

#[ApiErrorResponse]
pub enum IntrospectionErrorResponse {
    #[status_code(StatusCode::BAD_REQUEST)]
    #[error("invalid_request")]
    #[description(
        "The request is missing a required parameter, includes an invalid parameter value, or is otherwise malformed."
    )]
    InvalidRequestBody,

    #[status_code(axum::http::StatusCode::INTERNAL_SERVER_ERROR)]
    #[error("server_error")]
    #[description("Internal server error.")]
    DatabaseError,
}
