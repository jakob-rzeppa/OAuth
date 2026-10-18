use api_macros::ApiResponse;

use crate::domain::projection::user::FullUserProjection;

#[ApiResponse(axum::http::StatusCode::OK)]
#[derive(utoipa::ToSchema)]
pub struct AuthenticateUserResponse {
    pub data: FullUserProjection,
}
