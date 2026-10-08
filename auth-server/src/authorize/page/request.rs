use axum::extract::{FromRequestParts, Query};
use serde::Deserialize;

use crate::authorize::error_page::AuthorizeErrorPage;

#[derive(Deserialize, FromRequestParts)]
#[from_request(via(Query), rejection(AuthorizeErrorPage))]
pub struct AuthorizePageQuery {
    pub client_id: Option<String>,
    pub request_uri: Option<String>,
}
