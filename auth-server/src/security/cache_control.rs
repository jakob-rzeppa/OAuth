use axum::{
    http::{HeaderValue, header},
    response::Response,
};

/// Response layer for endpoints whose responses carry secrets or per-user state (tokens,
/// introspection results, authorization responses with the `code` in the `Location` header, login
/// and consent pages): keeps them out of browser and intermediary caches. `Pragma: no-cache` is
/// for HTTP/1.0 caches, and RFC 6749 §5.1 requires it on token responses.
pub async fn no_store(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request, middleware::map_response, routing::get};
    use tower::ServiceExt;

    #[tokio::test]
    async fn sets_no_store_and_pragma_no_cache() {
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(map_response(no_store));

        let response = app
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
        assert_eq!(response.headers().get(header::PRAGMA).unwrap(), "no-cache");
    }
}
