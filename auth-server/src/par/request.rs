use axum::{Json, extract::FromRequest};
use serde::Deserialize;

use crate::par::error_response::ParErrorResponse;

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json), rejection(ParErrorResponse))]
pub struct ParRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}
