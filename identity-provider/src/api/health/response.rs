use api_macros::ApiResponse;
use axum::http::StatusCode;

#[ApiResponse(StatusCode::OK)]
#[derive(utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: String,
}
