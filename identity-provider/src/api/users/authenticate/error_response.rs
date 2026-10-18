use api_macros::ApiErrorResponse;
use axum::http::StatusCode;

#[ApiErrorResponse]
pub enum AuthenticateUserErrorResponse {
    #[status_code(StatusCode::BAD_REQUEST)]
    #[error("invalid_request")]
    #[description("Invalid request body.")]
    InvalidBody,

    #[status_code(StatusCode::NOT_FOUND)]
    #[error("user_not_found")]
    #[description("No user with this user name exists.")]
    UserNotFound,

    #[status_code(StatusCode::UNAUTHORIZED)]
    #[error("unauthorized")]
    #[description("The provided credentials are invalid.")]
    Unauthorized,

    #[status_code(StatusCode::INTERNAL_SERVER_ERROR)]
    #[error("internal_server_error")]
    #[description("An internal server error occurred.")]
    InternalServerError,
}
