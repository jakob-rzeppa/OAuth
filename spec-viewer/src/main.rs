use std::net::SocketAddr;

use axum::Router;
use tokio::net::TcpListener;

mod web;

const DEFAULT_PORT: u16 = 3000;

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT));
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
    Router::new().merge(web::router())
}
