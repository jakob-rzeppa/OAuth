use axum::extract::FromRequestParts;
use serde::Deserialize;

use crate::{authorize::error_page::AuthorizeErrorPage, util::extract::parse_query};

#[derive(Deserialize)]
pub struct AuthorizePageQuery {
    pub client_id: Option<String>,
    pub request_uri: Option<String>,
}

impl<S: Send + Sync> FromRequestParts<S> for AuthorizePageQuery {
    type Rejection = AuthorizeErrorPage;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parse_query(parts).ok_or(AuthorizeErrorPage::MalformedRequest)
    }
}
