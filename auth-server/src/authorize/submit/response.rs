use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use url::Url;

use crate::authorize::error_page::AuthorizeErrorPage;

pub struct AuthorizeSubmitResponse {
    pub code: String,
    pub redirect_uri: String,
    pub state: String,
    pub expires_in: u64,
    pub iss: String,
}

impl IntoResponse for AuthorizeSubmitResponse {
    fn into_response(self) -> Response {
        let Ok(mut url) = Url::parse(&self.redirect_uri) else {
            return AuthorizeErrorPage::ServerError.into_response();
        };
        url.query_pairs_mut()
            .append_pair("code", &self.code)
            .append_pair("state", &self.state)
            .append_pair("expires_in", &self.expires_in.to_string())
            .append_pair("iss", &self.iss);
        (StatusCode::FOUND, [(header::LOCATION, url.as_str())]).into_response()
    }
}
