use api_macros::ApiRequest;
use uuid::Uuid;

use crate::api::users::update::error_response::UpdateUserErrorResponse;

#[ApiRequest(UpdateUserErrorResponse::InvalidBody)]
#[derive(utoipa::ToSchema)]
pub struct UpdateUserRequest {
    pub user_name: Option<String>,
    pub display_name: Option<String>,
    pub role_ids: Option<Vec<Uuid>>,
}
