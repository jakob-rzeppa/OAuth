use std::net::SocketAddr;

use tokio::net::TcpListener;

use crate::config::CONFIG;

mod api;
mod application;
mod config;
mod domain;
mod logging;
mod persistence;

#[tokio::main]
async fn main() {
    logging::init(CONFIG.log_level());

    tracing::info!("application starting");
    let app = api::router();

    // Specify the address to bind to (0.0.0.0 to listen on all interfaces)
    let addr = SocketAddr::from(([0, 0, 0, 0], CONFIG.app_port()));

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
