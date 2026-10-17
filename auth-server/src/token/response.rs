use api_macros::ApiResponse;
use axum::http::StatusCode;

#[ApiResponse(StatusCode::OK)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: String,
    /// Only issued if the `openid` scope was granted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
}
