use api_macros::ApiResponse;
use axum::http::StatusCode;
use uuid::Uuid;

/// The claims about the user (OpenID Connect Core 1.0, section 5.3.2). Claims of scopes that were
/// not granted are left out.
#[ApiResponse(StatusCode::OK)]
pub struct UserInfoResponse {
    /// Always present, and the same as the `sub` of the ID token.
    pub sub: Uuid,

    // `profile` scope
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_username: Option<String>,
}
