use api_macros::ApiResponse;
use axum::http::StatusCode;

#[ApiResponse(StatusCode::OK)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: String,
}
