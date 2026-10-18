use api_macros::ApiErrorResponse;
use axum::http::StatusCode;

#[ApiErrorResponse]
pub enum CreateUserErrorResponse {
    #[status_code(StatusCode::BAD_REQUEST)]
    #[error("invalid_request")]
    #[description("Invalid request body.")]
    InvalidBody,

    #[status_code(StatusCode::BAD_REQUEST)]
    #[error("invalid_user_name")]
    #[description("The user name is invalid.")]
    InvalidUserName,

    #[status_code(StatusCode::CONFLICT)]
    #[error("user_name_already_exists")]
    #[description("User name already exists.")]
    UserNameAlreadyExists,

    #[status_code(StatusCode::INTERNAL_SERVER_ERROR)]
    #[error("internal_server_error")]
    #[description("An internal server error occurred.")]
    InternalServerError,
}
