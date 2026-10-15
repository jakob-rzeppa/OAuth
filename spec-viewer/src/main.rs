use std::{net::SocketAddr, sync::Arc};

use axum::{Router, routing::get};
use tokio::net::TcpListener;

use crate::{
    application::{spec_list::SpecListService, spec_view::SpecViewService},
    client::{GrpcSpecGateway, HttpContentGateway},
    config::Config,
    state::AppState,
};

mod application;
mod client;
mod config;
mod state;
mod web;

#[tokio::main]
async fn main() {
    let config = Config::from_env().unwrap_or_else(|error| exit_with(error));
    let spec_gateway = Arc::new(
        GrpcSpecGateway::connect_lazy(&config.spec_provider_url)
            .unwrap_or_else(|error| exit_with(error)),
    );
    let content_gateway = Arc::new(HttpContentGateway::new(&config.content_storage_url));
    let state = AppState::new(
        SpecViewService::new(spec_gateway.clone(), content_gateway),
        SpecListService::new(spec_gateway),
    );

    let addr = SocketAddr::from(([0, 0, 0, 0], config.app_port));
    let listener = TcpListener::bind(addr).await.unwrap_or_else(|error| {
        eprintln!("failed to bind {addr}: {error}");
        std::process::exit(1)
    });

    println!("spec-viewer listening on {addr}");
    if let Err(error) = axum::serve(listener, app(state)).await {
        eprintln!("server stopped unexpectedly: {error}");
        std::process::exit(1);
    }
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(web::router())
        .with_state(state)
}

fn exit_with(error: impl std::fmt::Display) -> ! {
    eprintln!("{error}");
    std::process::exit(1)
}
