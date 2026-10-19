use std::net::SocketAddr;

use axum::{
    Router,
    body::Body,
    http::Request,
    routing::get,
};
use tower_http::trace::{DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::config::Config;

mod config;
mod domain;
mod error;
mod logging;
mod persistence;
mod security;
mod session;
mod tls;
mod util;

mod authorize;
mod discovery;
mod introspection;
mod jwks;
mod login;
mod par;
mod token;
mod userinfo;

#[tokio::main]
async fn main() {
    logging::init(Config::log_level());

    tracing::info!("application starting");
    Config::init();
    jwks::init();
    let app = app();

    // Specify the address to bind to (0.0.0.0 to listen on all interfaces)
    let addr = SocketAddr::from(([0, 0, 0, 0], Config::app_port()));

    let tls = tls::load().await;

    tracing::info!(%addr, "server listening (https)");
    if let Err(error) = axum_server::bind_rustls(addr, tls)
        .serve(app.into_make_service())
        .await
    {
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
        .merge(discovery::router())
        .merge(jwks::router())
        .merge(userinfo::router())
        .merge(authorize::router())
        .merge(login::router())
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

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
