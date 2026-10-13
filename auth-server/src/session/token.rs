use std::marker::PhantomData;

use axum::{
    extract::OptionalFromRequestParts,
    http::{HeaderMap, header},
};

use crate::session::{Session, cookie::cookie_value};

/// The token of a session of kind `S`, read from its cookie. Never logged, only its hash is stored.
pub struct SessionToken<S> {
    token: String,
    kind: PhantomData<S>,
}

impl<S: Session> SessionToken<S> {
    #[cfg(test)]
    pub fn new(token: String) -> Self {
        Self {
            token,
            kind: PhantomData,
        }
    }

    /// Finds the session cookie among the `Cookie` headers.
    pub fn from_headers(headers: &HeaderMap) -> Option<Self> {
        headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find_map(|cookies| cookie_value(cookies, S::COOKIE_NAME))
            .map(|token| Self {
                token: token.to_string(),
                kind: PhantomData,
            })
    }

    pub fn as_str(&self) -> &str {
        &self.token
    }
}

impl<S: Session, State: Send + Sync> OptionalFromRequestParts<State> for SessionToken<S> {
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
    use crate::domain::entity::{login_session::LoginSession, user_session::UserSession};

    fn parts(cookie: Option<&str>) -> axum::http::request::Parts {
        let mut request = Request::post("/");
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        request.body(()).unwrap().into_parts().0
    }

    #[tokio::test]
    async fn extractor_reads_the_token_from_the_cookie_of_its_kind() {
        let mut parts = parts(Some(
            "__Host-login_session=login-token; __Host-user_session=user-token",
        ));

        let login =
            <SessionToken<LoginSession> as OptionalFromRequestParts<()>>::from_request_parts(
                &mut parts,
                &(),
            )
            .await;
        let user = <SessionToken<UserSession> as OptionalFromRequestParts<()>>::from_request_parts(
            &mut parts,
            &(),
        )
        .await;

        assert_eq!(login.unwrap().unwrap().as_str(), "login-token");
        assert_eq!(user.unwrap().unwrap().as_str(), "user-token");
    }

    #[tokio::test]
    async fn extractor_yields_none_without_the_cookie() {
        let mut parts = parts(Some("theme=dark"));

        let token =
            <SessionToken<UserSession> as OptionalFromRequestParts<()>>::from_request_parts(
                &mut parts,
                &(),
            )
            .await;

        assert!(token.unwrap().is_none());
    }

    #[test]
    fn ignores_a_cookie_without_the_host_prefix() {
        let parts = parts(Some("user_session=forged"));

        assert!(SessionToken::<UserSession>::from_headers(&parts.headers).is_none());
    }
}
