use axum::{Json, extract::FromRequest};
use serde::Deserialize;

use crate::introspection::error_response::IntrospectionErrorResponse;

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json), rejection(IntrospectionErrorResponse))]
pub struct IntrospectionRequest {
    pub token: String,
}
