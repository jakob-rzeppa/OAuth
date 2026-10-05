use std::net::SocketAddr;

use axum::{
    Router,
    http::{HeaderValue, header},
    middleware::map_response,
    response::Response,
};
use tokio::net::TcpListener;

mod api;
mod config;
mod domain;
mod persistence;
mod util;
mod web;

#[tokio::main]
async fn main() {
    println!("[STARTUP] Application starting...");

    println!("[STARTUP] Building router...");
    let app = app();

    // Specify the address to bind to (0.0.0.0 to listen on all interfaces)
    let addr = SocketAddr::from(([0, 0, 0, 0], config::app_port()));

    // Create listener on address
    println!("[STARTUP] Binding to address: {}", addr);
    let listener = TcpListener::bind(addr)
        .await
        .expect(format!("[STARTUP] Failed to create TCP listener: {}", addr).as_str());

    // Start the Axum server
    println!("[STARTUP] Server running at {}", addr);
    axum::serve(listener, app)
        .await
        .expect("[STARTUP] Failed to launch server");
}

fn app() -> Router {
    api::router()
        .merge(web::router())
        .layer(map_response(set_cache_control))
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
}
