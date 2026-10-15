use std::net::SocketAddr;

use axum::{Router, routing::get};
use tokio::net::TcpListener;

use crate::config::Config;

mod application;
mod config;
mod content;
mod spec;
mod web;

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], Config::app_port()));
    let listener = TcpListener::bind(addr).await.unwrap_or_else(|error| {
        eprintln!("failed to bind {addr}: {error}");
        std::process::exit(1)
    });

    println!("spec-viewer listening on {addr}");
    if let Err(error) = axum::serve(listener, app()).await {
        eprintln!("server stopped unexpectedly: {error}");
        std::process::exit(1);
    }
}

fn app() -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(web::router())
}
