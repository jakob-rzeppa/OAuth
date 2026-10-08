mod error_page;
mod page;
mod submit;

use axum::{Router, middleware::map_response, routing::get};

use crate::{
    authorize::{page::authorize_page_endpoint, submit::authorize_submit_endpoint},
    util::headers::{prevent_framing, set_referrer_policy},
};

pub fn router() -> Router {
    Router::new()
        .route(
            "/authorize",
            get(authorize_page_endpoint).post(authorize_submit_endpoint),
        )
        .layer(map_response(set_referrer_policy))
        .layer(map_response(prevent_framing))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, header},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn sets_referrer_policy() {
        // Without query parameters the consent page responds with an error page,
        // before touching any persistence.
        let request = Request::get("/authorize").body(Body::empty()).unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert_eq!(
            response.headers().get(header::REFERRER_POLICY).unwrap(),
            "no-referrer"
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
}
