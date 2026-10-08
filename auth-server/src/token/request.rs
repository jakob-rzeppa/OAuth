use api_macros::ApiRequest;

use crate::token::error_response::TokenErrorResponse;

#[ApiRequest(TokenErrorResponse::InvalidRequestBody)]
pub struct TokenRequest {
    pub grant_type: String,
    pub client_id: String,

    // Authorization Code Grant
    pub code: Option<String>,
    pub code_verifier: Option<String>,
}
