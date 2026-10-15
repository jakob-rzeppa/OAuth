//! Standalone mock of the content storage for developing the spec-viewer by hand.
//!
//! Run with `cargo run --example mock_content` next to `cargo run --example mock_provider`,
//! whose `content_uri`s point here.
//!
//! It listens on `127.0.0.1:50052`. Override the address with `MOCK_CONTENT_ADDR`, a socket
//! address like `0.0.0.0:50052`. A spec-viewer in docker on Linux only reaches it on an
//! address other than the loopback one.
//!
//! Serves the full text of every patch and proposal of the mock provider's known specs, at
//! `/{spec}/{patch or proposal}.md`. Anything else is `404`.

use axum::{
    Router,
    extract::Path,
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::get,
};
use tokio::net::TcpListener;

const DEFAULT_ADDR: &str = "127.0.0.1:50052";

/// The texts by their path.
const TEXTS: &[(&str, &str)] = &[
    (
        "example-spec/patch-1.md",
        include_str!("texts/example-spec/patch-1.md"),
    ),
    (
        "example-spec/patch-2.md",
        include_str!("texts/example-spec/patch-2.md"),
    ),
    (
        "example-spec/patch-3.md",
        include_str!("texts/example-spec/patch-3.md"),
    ),
    (
        "example-draft/patch-1.md",
        include_str!("texts/example-draft/patch-1.md"),
    ),
    (
        "example-draft/patch-2.md",
        include_str!("texts/example-draft/patch-2.md"),
    ),
    (
        "example-draft/proposal-1.md",
        include_str!("texts/example-draft/proposal-1.md"),
    ),
    (
        "example-draft/proposal-2.md",
        include_str!("texts/example-draft/proposal-2.md"),
    ),
];

async fn get_text(Path(path): Path<String>) -> Response {
    println!("get_text({path})");

    match TEXTS.iter().find(|(known, _)| *known == path) {
        Some((_, text)) => {
            ([(CONTENT_TYPE, "text/markdown; charset=utf-8")], *text).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[tokio::main]
async fn main() {
    let addr = std::env::var("MOCK_CONTENT_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.into());
    let listener = TcpListener::bind(&addr).await.unwrap_or_else(|error| {
        eprintln!("failed to bind {addr}: {error}");
        std::process::exit(1)
    });

    println!("mock content storage listening on {addr}");
    let app = Router::new().route("/{*path}", get(get_text));
    if let Err(error) = axum::serve(listener, app).await {
        eprintln!("mock content storage stopped unexpectedly: {error}");
        std::process::exit(1);
    }
}
