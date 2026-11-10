use axum::{Form, extract::FromRequest};
use serde::Deserialize;

use crate::endpoints::introspection::error::IntrospectionError;

#[derive(Deserialize, FromRequest)]
#[from_request(via(Form), rejection(IntrospectionError))]
pub struct IntrospectionRequest {
    pub token: String,
}
