mod error_page;
mod page;
mod submit;

use axum::{
    Router,
    middleware::{from_fn, map_response},
    routing::get,
};

use crate::{
    authorize::{page::authorize_page_endpoint, submit::authorize_submit_endpoint},
    security::{cache_control::no_store, fetch_metadata::reject_cross_site},
    util::headers::{prevent_framing, set_referrer_policy},
};

pub fn router() -> Router {
    Router::new()
        .route(
            "/authorize",
            get(authorize_page_endpoint).post(authorize_submit_endpoint),
        )
        .layer(from_fn(reject_cross_site))
        .layer(map_response(no_store))
        .layer(map_response(set_referrer_policy))
        .layer(map_response(prevent_framing))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn sends_no_store() {
        // Without query parameters the page responds with an error page,
        // before touching any persistence.
        let request = Request::get("/authorize").body(Body::empty()).unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
    }

    #[tokio::test]
    async fn sets_referrer_policy() {
        // Without query parameters the consent page responds with an error page,
        // before touching any persistence.
        let request = Request::get("/authorize").body(Body::empty()).unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert_eq!(
            response.headers().get(header::REFERRER_POLICY).unwrap(),
            "same-origin"
        );
    }

    #[tokio::test]
    async fn prevents_framing() {
        // Without query parameters the consent page responds with an error page,
        // before touching any persistence.
        let request = Request::get("/authorize").body(Body::empty()).unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_SECURITY_POLICY)
                .unwrap(),
            "frame-ancestors 'none'"
        );
        assert_eq!(
            response.headers().get(header::X_FRAME_OPTIONS).unwrap(),
            "DENY"
        );
    }

    #[tokio::test]
    async fn rejects_cross_site_posts_with_a_framing_protected_error_page() {
        // Rejected by the middleware, before touching any persistence.
        let request = Request::post("/authorize")
            .header("sec-fetch-site", "cross-site")
            .body(Body::empty())
            .unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let headers = response.headers().clone();
        assert_eq!(headers.get(header::REFERRER_POLICY).unwrap(), "same-origin");
        assert_eq!(headers.get(header::X_FRAME_OPTIONS).unwrap(), "DENY");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("<h1>invalid_request</h1>"), "{body}");
    }

    #[tokio::test]
    async fn rejects_posts_from_another_origin() {
        Config::iss_fake().setup(|| "https://issuer.example");
        let request = Request::post("/authorize")
            .header("origin", "https://evil.example")
            .body(Body::empty())
            .unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn rejects_posts_without_origin_information() {
        let request = Request::post("/authorize").body(Body::empty()).unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("up-to-date browser"), "{body}");
    }
}
