use std::net::SocketAddr;

use axum::{
    Router,
    body::Body,
    http::{HeaderValue, Request, header},
    middleware::map_response,
    response::Response,
    routing::get,
};
use tokio::net::TcpListener;
use tower_http::trace::{DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;

mod config;
mod domain;
mod logging;
mod persistence;
mod security;
mod util;

mod authorize;
mod introspection;
mod login;
mod par;
mod token;

#[tokio::main]
async fn main() {
    logging::init(config::log_level());

    tracing::info!("application starting");
    let app = app();

    // Specify the address to bind to (0.0.0.0 to listen on all interfaces)
    let addr = SocketAddr::from(([0, 0, 0, 0], config::app_port()));

    // Create listener on address
    let listener = TcpListener::bind(addr).await.unwrap_or_else(|error| {
        tracing::error!(%addr, ?error, "failed to bind the TCP listener");
        std::process::exit(1)
    });

    // Start the Axum server
    tracing::info!(%addr, "server listening");
    if let Err(error) = axum::serve(listener, app).await {
        tracing::error!(?error, "server stopped unexpectedly");
        std::process::exit(1);
    }
}

fn app() -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(par::router())
        .merge(token::router())
        .merge(introspection::router())
        .merge(authorize::router())
        .merge(login::router())
        .layer(map_response(set_cache_control))
        // Outermost, so it sees the final response. Only the method and path are recorded: the
        // query string of `/authorize` carries the `request_uri`, which must not end up in the log.
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

/// Keep every response out of browser and intermediary caches. They carry tokens,
/// introspection results, and authorization responses (`code` in the `Location` header).
async fn set_cache_control(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn sets_cache_control_on_api_responses() {
        let request = Request::get("/health").body(Body::empty()).unwrap();

        let response = app().oneshot(request).await.unwrap();

        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
    }

    #[tokio::test]
    async fn sets_cache_control_on_api_error_responses() {
        // An invalid body is rejected before touching any persistence.
        let request = Request::post("/token")
            .body(Body::from("not json"))
            .unwrap();

        let response = app().oneshot(request).await.unwrap();

        assert_eq!(response.status(), 400);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
    }

    #[tokio::test]
    async fn sets_cache_control_on_web_responses() {
        // Without query parameters the consent page responds with an error page,
        // before touching any persistence.
        let request = Request::get("/authorize").body(Body::empty()).unwrap();

        let response = app().oneshot(request).await.unwrap();

        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
    }

    #[tokio::test]
    async fn logs_requests_at_debug_with_the_path_but_without_the_query_string() {
        let capture = logging::testing::LogCapture::default();
        let _guard = capture.install();

        // The invalid client_id is rejected before touching any persistence.
        let request =
            Request::get("/authorize?client_id=not-a-uuid&request_uri=urn:secret-request-uri")
                .body(Body::empty())
                .unwrap();
        app().oneshot(request).await.unwrap();

        let log = capture.contents();
        assert!(log.contains("path=/authorize"), "{log}");
        assert!(log.contains("started processing request"), "{log}");
        assert!(log.contains("finished processing request"), "{log}");
        assert!(!log.contains("secret-request-uri"), "{log}");
        assert!(!log.contains("not-a-uuid"), "{log}");
    }
}
