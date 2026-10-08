use api_macros::ApiRequest;

use crate::par::error_response::AuthorizePushErrorResponse;

#[ApiRequest(AuthorizePushErrorResponse::InvalidRequestBody)]
pub struct AuthorizePushRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}
