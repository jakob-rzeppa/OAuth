use std::{net::SocketAddr, sync::Arc};

use spec_api::spec_provider_server::SpecProviderServer;
use tonic::transport::Server;

use crate::{
    api::SpecProviderService,
    application::spec_service::SpecService,
    persistence::in_memory::{
        draft_repository::InMemoryDraftRepository, spec_repository::InMemorySpecRepository,
    },
};

mod api;
mod application;
mod domain;
mod persistence;

const DEFAULT_PORT: u16 = 50051;

#[tokio::main]
async fn main() {
    println!("[STARTUP] Application starting...");

    let spec_service = SpecService::new(
        Arc::new(InMemorySpecRepository::default()),
        Arc::new(InMemoryDraftRepository::default()),
    );
    let spec_provider_service = SpecProviderService::new(spec_service);

    let port = std::env::var("APP_PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("[STARTUP] Server running at {}", addr);
    Server::builder()
        .add_service(SpecProviderServer::new(spec_provider_service))
        .serve_with_shutdown(addr, shutdown_signal())
        .await
        .expect("[STARTUP] Failed to launch server");

    println!("[SHUTDOWN] Server stopped");
}

/// Resolves on Ctrl+C or SIGTERM (sent by `docker stop`).
/// The server then stops accepting connections and waits for in-flight requests to finish.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("[SHUTDOWN] Failed to listen for Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("[SHUTDOWN] Failed to listen for SIGTERM")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("[SHUTDOWN] Signal received, shutting down gracefully...");
}
