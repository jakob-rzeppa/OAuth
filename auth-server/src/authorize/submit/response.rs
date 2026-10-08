use axum::response::{IntoResponse, Redirect, Response};
use url::Url;

use crate::{authorize::error_page::AuthorizeErrorPage, error::InternalError};

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
            return AuthorizeErrorPage::ServerError(InternalError::Invariant(
                "a validated redirect_uri is not a valid URL",
            ))
            .into_response();
        };
        url.query_pairs_mut()
            .append_pair("code", &self.code)
            .append_pair("state", &self.state)
            .append_pair("expires_in", &self.expires_in.to_string())
            .append_pair("iss", &self.iss);
        // 303, so the browser follows up with a GET even though this was a POST.
        Redirect::to(url.as_str()).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{StatusCode, header};

    #[test]
    fn redirects_to_the_client_with_see_other() {
        let response = AuthorizeSubmitResponse {
            code: "the-code".to_string(),
            redirect_uri: "https://example.com/callback".to_string(),
            state: "the-state".to_string(),
            expires_in: 300,
            iss: "https://issuer.example".to_string(),
        }
        .into_response();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let location = response.headers()[header::LOCATION].to_str().unwrap();
        assert!(
            location.starts_with("https://example.com/callback?code=the-code&state=the-state"),
            "{location}"
        );
    }
}
