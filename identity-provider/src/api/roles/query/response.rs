use api_macros::ApiResponse;
use axum::http::StatusCode;

use crate::domain::projection::role::FullRoleProjection;

#[ApiResponse(StatusCode::OK)]
#[derive(utoipa::ToSchema)]
pub struct QueryRolesResponse {
    pub roles: Vec<FullRoleProjection>,
}
