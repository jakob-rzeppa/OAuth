use axum::{
    extract::OptionalFromRequestParts,
    http::{HeaderMap, header},
};

use crate::session::{COOKIE_NAME, cookie::cookie_value};

/// The token of the user session, read from its cookie. Never logged, only its hash is stored.
pub struct SessionToken {
    token: String,
}

impl SessionToken {
    #[cfg(test)]
    pub fn new(token: String) -> Self {
        Self { token }
    }

    /// Finds the session cookie among the `Cookie` headers.
    pub fn from_headers(headers: &HeaderMap) -> Option<Self> {
        headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find_map(|cookies| cookie_value(cookies, COOKIE_NAME))
            .map(|token| Self {
                token: token.to_string(),
            })
    }

    pub fn as_str(&self) -> &str {
        &self.token
    }
}

impl<State: Send + Sync> OptionalFromRequestParts<State> for SessionToken {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &State,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(Self::from_headers(&parts.headers))
    }
}

#[cfg(test)]
mod tests {
    use axum::extract::Request;

    use super::*;

    fn parts(cookie: Option<&str>) -> axum::http::request::Parts {
        let mut request = Request::post("/");
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        request.body(()).unwrap().into_parts().0
    }

    #[tokio::test]
    async fn extractor_reads_the_token_from_the_session_cookie() {
        let mut parts = parts(Some("theme=dark; __Host-user_session=user-token"));

        let user =
            <SessionToken as OptionalFromRequestParts<()>>::from_request_parts(&mut parts, &())
                .await;

        assert_eq!(user.unwrap().unwrap().as_str(), "user-token");
    }

    #[tokio::test]
    async fn extractor_yields_none_without_the_cookie() {
        let mut parts = parts(Some("theme=dark"));

        let token =
            <SessionToken as OptionalFromRequestParts<()>>::from_request_parts(&mut parts, &())
                .await;

        assert!(token.unwrap().is_none());
    }

    #[test]
    fn ignores_a_cookie_without_the_host_prefix() {
        let parts = parts(Some("user_session=forged"));

        assert!(SessionToken::from_headers(&parts.headers).is_none());
    }
}
