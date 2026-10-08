use api_macros::ApiResponse;
use axum::http::StatusCode;

#[ApiResponse(StatusCode::CREATED)]
pub struct ParResponse {
    pub request_uri: String,
    pub expires_in: u64,
}
