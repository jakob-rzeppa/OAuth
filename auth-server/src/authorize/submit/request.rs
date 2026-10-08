use axum::extract::{FromRequest, Request};
use serde::Deserialize;
use uuid::Uuid;

use crate::{authorize::error_page::AuthorizeErrorPage, util::extract::parse_form};

#[derive(Deserialize)]
pub struct AuthorizeSubmitRequest {
    pub request_uri: String,
    pub client_id: Uuid,
    pub decision: bool,
}

impl<S: Send + Sync> FromRequest<S> for AuthorizeSubmitRequest {
    type Rejection = AuthorizeErrorPage;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        parse_form(request, state)
            .await
            .ok_or(AuthorizeErrorPage::MalformedRequest)
    }
}
