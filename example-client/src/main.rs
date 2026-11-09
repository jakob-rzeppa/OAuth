use std::net::SocketAddr;

use axum::{Router, http::StatusCode, routing::get};
use axum_server::tls_rustls::RustlsConfig;

use crate::{app_state::AppState};

mod app_state;
mod config;
mod security;
mod web;

#[tokio::main]
async fn main() {
    config::init();

    let app = Router::new()
        .route("/", get(web::index))
        .nest("/flow", web::router())
        // Used by the compose healthcheck.
        .route("/health", get(|| async { StatusCode::OK }))
        .with_state(AppState::default());

    let port: u16 = std::env::var("APP_PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // The server only answers over https, like the other servers (see DEVELOPMENT.md).
    let (cert_path, key_path) = (config::tls_cert_path(), config::tls_key_path());
    let tls = RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .unwrap_or_else(|error| {
            panic!("failed to load the TLS certificate ({cert_path}, {key_path}): {error}");
        });

    println!("Listening on https://{addr}");
    if let Err(error) = axum_server::bind_rustls(addr, tls)
        .serve(app.into_make_service())
        .await
    {
        panic!("server stopped unexpectedly: {error}");
    }
}
