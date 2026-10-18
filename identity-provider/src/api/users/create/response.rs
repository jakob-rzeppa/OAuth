use api_macros::ApiResponse;
use axum::http::StatusCode;
use uuid::Uuid;

#[ApiResponse(StatusCode::CREATED)]
#[derive(utoipa::ToSchema)]
pub struct CreateUserResponse {
    pub id: Uuid,
    pub temporary_password: String,
}
