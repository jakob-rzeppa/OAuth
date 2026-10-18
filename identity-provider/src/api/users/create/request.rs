use api_macros::ApiRequest;
use uuid::Uuid;

use crate::api::users::create::error_response::CreateUserErrorResponse;

#[ApiRequest(CreateUserErrorResponse::InvalidBody)]
#[derive(utoipa::ToSchema)]
pub struct CreateUserRequest {
    pub user_name: String,
    pub role_ids: Vec<Uuid>,
}
