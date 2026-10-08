use askama::Template;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::util::html::render_html;

#[derive(Template)]
#[template(path = "authorize.html")]
pub struct AuthorizePageResponse {
    pub client_name: String,
    pub scope: String,
    pub client_id: Uuid,
    pub request_uri: String,
}

impl IntoResponse for AuthorizePageResponse {
    fn into_response(self) -> Response {
        render_html(&self, StatusCode::OK)
    }
}
