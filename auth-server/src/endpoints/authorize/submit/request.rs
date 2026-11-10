use axum::{Form, extract::FromRequest};
use serde::Deserialize;
use uuid::Uuid;

use crate::endpoints::authorize::error::AuthorizeError;

#[derive(Deserialize, FromRequest)]
#[from_request(via(Form), rejection(AuthorizeError))]
pub struct AuthorizeSubmitRequest {
    pub request_uri: String,
    pub client_id: Uuid,
    pub decision: bool,
}
