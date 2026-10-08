use axum::{Form, extract::FromRequest};
use serde::Deserialize;
use uuid::Uuid;

use crate::authorize::error_page::AuthorizeErrorPage;

#[derive(Deserialize, FromRequest)]
#[from_request(via(Form), rejection(AuthorizeErrorPage))]
pub struct AuthorizeSubmitRequest {
    pub request_uri: String,
    pub client_id: Uuid,
    pub decision: bool,
}
