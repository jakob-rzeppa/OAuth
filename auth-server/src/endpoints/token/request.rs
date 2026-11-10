use axum::{Form, extract::FromRequest};
use serde::Deserialize;

use crate::endpoints::token::error::TokenError;

#[derive(Deserialize, FromRequest)]
#[from_request(via(Form), rejection(TokenError))]
pub struct TokenRequest {
    pub grant_type: String,
    pub client_id: String,

    // Authorization Code Grant
    pub code: Option<String>,
    pub code_verifier: Option<String>,
}
