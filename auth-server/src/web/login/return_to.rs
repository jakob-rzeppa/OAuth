use axum::extract::FromRequestParts;
use serde::Deserialize;

use crate::web::login::error_response::LoginErrorResponse;

#[derive(Deserialize)]
pub struct ReturnToQuery {
    pub return_to: String,
}

impl<S: Send + Sync> FromRequestParts<S> for ReturnToQuery {
    type Rejection = LoginErrorResponse;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let query = parts
            .uri
            .query()
            .ok_or(LoginErrorResponse::InvalidReturnTo)?;
        serde_urlencoded::from_str(query).map_err(|_| LoginErrorResponse::MalformedRequest)
    }
}

/// Only paths on this server are allowed as `return_to`, so the login can't be used as an open
/// redirect. `//host` and `/\host` are rejected because browsers treat them as protocol-relative.
pub(super) fn is_local_path(return_to: &str) -> bool {
    return_to.starts_with('/')
        && !return_to.starts_with("//")
        && !return_to.contains(['\\', '\r', '\n'])
}

#[cfg(test)]
mod tests {
    use axum::http::Request;

    use super::*;

    async fn extract(uri: &str) -> Result<ReturnToQuery, LoginErrorResponse> {
        let (mut parts, _) = Request::get(uri).body(()).unwrap().into_parts();
        ReturnToQuery::from_request_parts(&mut parts, &()).await
    }

    #[tokio::test]
    async fn extractor_reads_the_decoded_return_to_from_the_query() {
        let query = extract("/login?return_to=%2Fauthorize%3Fclient_id%3D1").await;

        assert_eq!(query.ok().unwrap().return_to, "/authorize?client_id=1");
    }

    #[tokio::test]
    async fn extractor_fails_with_invalid_return_to_without_a_query() {
        let query = extract("/login").await;

        assert!(matches!(query, Err(LoginErrorResponse::InvalidReturnTo)));
    }

    #[tokio::test]
    async fn extractor_fails_with_malformed_request_without_return_to_in_the_query() {
        let query = extract("/login?other=1").await;

        assert!(matches!(query, Err(LoginErrorResponse::MalformedRequest)));
    }

    #[test]
    fn accepts_paths_on_this_server() {
        assert!(is_local_path(
            "/authorize?client_id=1&request_uri=urn%3Aabc"
        ));
    }

    #[test]
    fn rejects_return_to_values_leaving_this_server() {
        for return_to in [
            "",
            "https://evil.example",
            "//evil.example",
            "/\\evil.example",
            "authorize",
            "/ok\r\nSet-Cookie: a=b",
        ] {
            assert!(!is_local_path(return_to), "{return_to:?}");
        }
    }
}
