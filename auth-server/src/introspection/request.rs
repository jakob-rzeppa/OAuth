use axum::{Form, extract::FromRequest};
use serde::Deserialize;

use crate::introspection::error_response::IntrospectionErrorResponse;

#[derive(Deserialize, FromRequest)]
#[from_request(via(Form), rejection(IntrospectionErrorResponse))]
pub struct IntrospectionRequest {
    pub token: String,
}
