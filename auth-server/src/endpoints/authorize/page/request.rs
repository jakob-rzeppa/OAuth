use axum::extract::{FromRequestParts, Query};
use serde::Deserialize;

use crate::endpoints::authorize::error::AuthorizeError;

#[derive(Deserialize, FromRequestParts)]
#[from_request(via(Query), rejection(AuthorizeError))]
pub struct AuthorizePageQuery {
    pub client_id: Option<String>,
    pub request_uri: Option<String>,
}
