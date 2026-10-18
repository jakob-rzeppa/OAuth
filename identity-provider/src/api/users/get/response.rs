use api_macros::ApiResponse;
use axum::http::StatusCode;

use crate::domain::projection::user::FullUserProjection;

#[ApiResponse(StatusCode::OK)]
#[derive(utoipa::ToSchema)]
pub struct GetUserResponse {
    pub data: FullUserProjection,
}
