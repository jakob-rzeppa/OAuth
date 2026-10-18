use api_macros::ApiErrorResponse;
use axum::http::StatusCode;

#[ApiErrorResponse]
pub enum QueryUsersError {
    #[status_code(StatusCode::INTERNAL_SERVER_ERROR)]
    #[error("internal_server_error")]
    #[description("An internal server error occurred.")]
    InternalServerError,
}
