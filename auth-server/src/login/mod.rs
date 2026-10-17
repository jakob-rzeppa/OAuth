use axum::{
    Router,
    middleware::{from_fn, map_response},
    routing::get,
};

use crate::{
    login::{page::login_page_endpoint, submit::login_submit_endpoint},
    security::{cache_control::no_store, fetch_metadata::reject_cross_site},
    util::headers::{prevent_framing, set_referrer_policy},
};

mod error_response;
mod login_page;
mod page;
mod return_to;
mod submit;

pub fn router() -> Router {
    Router::new()
        .route(
            "/login",
            get(login_page_endpoint).post(login_submit_endpoint),
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
        let request = Request::get("/login").body(Body::empty()).unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
    }

    #[tokio::test]
    async fn sets_referrer_policy_and_prevents_framing() {
        // Without return_to the login page responds with an error page,
        // before touching any persistence.
        let request = Request::get("/login").body(Body::empty()).unwrap();

        let response = router().oneshot(request).await.unwrap();

        let headers = response.headers();
        assert_eq!(headers.get(header::REFERRER_POLICY).unwrap(), "same-origin");
        assert_eq!(
            headers.get(header::CONTENT_SECURITY_POLICY).unwrap(),
            "frame-ancestors 'none'"
        );
        assert_eq!(headers.get(header::X_FRAME_OPTIONS).unwrap(), "DENY");
    }

    #[tokio::test]
    async fn rejects_cross_site_posts_with_a_framing_protected_error_page() {
        // Rejected by the middleware, before touching any persistence.
        let request = Request::post("/login")
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
        let request = Request::post("/login")
            .header("origin", "https://evil.example")
            .body(Body::empty())
            .unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn rejects_posts_without_origin_information() {
        let request = Request::post("/login").body(Body::empty()).unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("up-to-date browser"), "{body}");
    }
}
