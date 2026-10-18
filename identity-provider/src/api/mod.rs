mod health;
mod openapi;
mod roles;
mod users;

use axum::{body::Body, http::Request};
use tower_http::trace::{DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use utoipa::OpenApi;

pub fn router() -> axum::Router {
    axum::Router::new()
        .route("/v1/health", axum::routing::get(health::health_endpoint))
        .nest("/v1/users", users::router())
        .nest("/v1/roles", roles::router())
        .merge(
            utoipa_swagger_ui::SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", openapi::ApiDoc::openapi()),
        )
        // Only the method and path are recorded, never the query string.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<Body>| {
                    tracing::debug_span!(
                        "request",
                        method = %request.method(),
                        path = %request.uri().path(),
                    )
                })
                .on_request(DefaultOnRequest::new().level(Level::DEBUG))
                .on_response(DefaultOnResponse::new().level(Level::DEBUG))
                .on_failure(DefaultOnFailure::new().level(Level::DEBUG)),
        )
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    use super::*;
    use crate::logging::testing::LogCapture;

    #[tokio::test]
    async fn logs_requests_at_debug_with_the_path_but_without_the_query_string() {
        let capture = LogCapture::default();
        let _guard = capture.install();

        // The health endpoint does not touch the database.
        let request = Request::get("/v1/health?token=secret-value")
            .body(Body::empty())
            .unwrap();
        let response = router().oneshot(request).await.unwrap();

        assert!(response.status().is_success());
        let log = capture.contents();
        assert!(log.contains("path=/v1/health"), "{log}");
        assert!(log.contains("started processing request"), "{log}");
        assert!(log.contains("finished processing request"), "{log}");
        assert!(!log.contains("secret-value"), "{log}");
    }
}
