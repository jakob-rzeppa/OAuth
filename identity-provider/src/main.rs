use std::net::SocketAddr;

use crate::config::CONFIG;

mod api;
mod application;
mod config;
mod domain;
mod logging;
mod persistence;
mod tls;

#[tokio::main]
async fn main() {
    logging::init(CONFIG.log_level());

    tracing::info!("application starting");
    let app = api::router();

    // Specify the address to bind to (0.0.0.0 to listen on all interfaces)
    let addr = SocketAddr::from(([0, 0, 0, 0], CONFIG.app_port()));

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
