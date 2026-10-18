use axum::response::{IntoResponse, Redirect, Response};
use url::Url;

use crate::{
    config::Config,
    error::{InternalError, SERVER_ERROR_DESCRIPTION, page::PageError},
    util::oauth_error::OAuthErrorCode,
};

/// An error reported back to the client by redirecting to its `redirect_uri`
/// (RFC 6749 §4.1.2.1), with the `iss` of RFC 9207. Only used once the client and its
/// `redirect_uri` are known to be valid; before that, errors are a [`PageError`].
#[derive(Debug)]
pub struct RedirectError {
    pub redirect_uri: String,
    pub state: String,
    pub reason: RedirectReason,
}

#[derive(Debug)]
pub enum RedirectReason {
    /// The code and the description are sent to the client.
    Described {
        code: OAuthErrorCode,
        description: &'static str,
    },
    /// Logged, and sent to the client as a generic `server_error`: once the redirect is
    /// committed to, even the server's own failures go back to the client.
    Server(InternalError),
}

impl RedirectError {
    pub fn new(
        code: OAuthErrorCode,
        description: &'static str,
        redirect_uri: String,
        state: String,
    ) -> Self {
        RedirectError {
            redirect_uri,
            state,
            reason: RedirectReason::Described { code, description },
        }
    }

    pub fn server(source: InternalError, redirect_uri: String, state: String) -> Self {
        RedirectError {
            redirect_uri,
            state,
            reason: RedirectReason::Server(source),
        }
    }
}

impl IntoResponse for RedirectError {
    fn into_response(self) -> Response {
        let (code, description) = match self.reason {
            RedirectReason::Described { code, description } => (code, description),
            RedirectReason::Server(source) => {
                source.log();
                (OAuthErrorCode::ServerError, SERVER_ERROR_DESCRIPTION)
            }
        };

        let Ok(mut url) = Url::parse(&self.redirect_uri) else {
            return PageError::Server(InternalError::Invariant(
                "a validated redirect_uri is not a valid URL",
            ))
            .into_response();
        };
        url.query_pairs_mut()
            .append_pair("error", code.as_str())
            .append_pair("error_description", description)
            .append_pair("state", &self.state)
            .append_pair("iss", Config::iss());
        Redirect::to(url.as_str()).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::testing::LogCapture;
    use axum::http::{StatusCode, header};

    const REDIRECT_URI: &str = "https://example.com/callback?client=param";

    fn query(response: &Response) -> Vec<(String, String)> {
        let location = Url::parse(response.headers()[header::LOCATION].to_str().unwrap()).unwrap();
        location.query_pairs().into_owned().collect()
    }

    fn pair(name: &str, value: &str) -> (String, String) {
        (name.to_string(), value.to_string())
    }

    #[test]
    fn redirects_with_the_code_description_state_and_iss() {
        Config::iss_fake().setup(|| "https://issuer.example");

        let response = RedirectError::new(
            OAuthErrorCode::AccessDenied,
            "The user denied the request.",
            REDIRECT_URI.to_string(),
            "the-state".to_string(),
        )
        .into_response();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            query(&response),
            vec![
                pair("client", "param"),
                pair("error", "access_denied"),
                pair("error_description", "The user denied the request."),
                pair("state", "the-state"),
                pair("iss", "https://issuer.example"),
            ]
        );
    }

    #[test]
    fn logs_a_server_error_and_redirects_without_showing_it() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        Config::iss_fake().setup(|| "https://issuer.example");

        let response = RedirectError::server(
            InternalError::Invariant("redis is down"),
            REDIRECT_URI.to_string(),
            "the-state".to_string(),
        )
        .into_response();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let query = query(&response);
        assert!(query.contains(&pair("error", "server_error")), "{query:?}");
        assert!(
            query.contains(&pair("error_description", SERVER_ERROR_DESCRIPTION)),
            "{query:?}"
        );
        assert!(!format!("{query:?}").contains("redis is down"));
        let log = capture.contents();
        assert!(log.contains("ERROR"), "{log}");
        assert!(log.contains("redis is down"), "{log}");
    }

    #[test]
    fn an_invalid_redirect_uri_becomes_a_server_error_page() {
        let capture = LogCapture::default();
        let _guard = capture.install();

        let response = RedirectError::new(
            OAuthErrorCode::AccessDenied,
            "The user denied the request.",
            "not a url".to_string(),
            "the-state".to_string(),
        )
        .into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(response.headers().get(header::LOCATION).is_none());
        assert!(capture.contents().contains("not a valid URL"));
    }
}
