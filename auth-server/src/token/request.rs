use axum::{Json, extract::FromRequest};
use serde::Deserialize;

use crate::token::error_response::TokenErrorResponse;

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json), rejection(TokenErrorResponse))]
pub struct TokenRequest {
    pub grant_type: String,
    pub client_id: String,

    // Authorization Code Grant
    pub code: Option<String>,
    pub code_verifier: Option<String>,
}
