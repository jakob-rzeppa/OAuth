use std::net::SocketAddr;

use askama::Template;
use axum::{
    Router,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use axum_server::tls_rustls::RustlsConfig;

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name: &'a str,
}

async fn hello() -> Response {
    match (HelloTemplate { name: "World" }).render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to render template: {err}"),
        )
            .into_response(),
    }
}

/// Reads a required environment variable, or exits.
fn require(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| {
        eprintln!("{name} must be set");
        std::process::exit(1)
    })
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(hello))
        // Used by the compose healthcheck.
        .route("/health", get(|| async { StatusCode::OK }));

    let port: u16 = std::env::var("APP_PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // The server only answers over https, like the other servers (see DEVELOPMENT.md).
    let (cert_path, key_path) = (require("TLS_CERT_PATH"), require("TLS_KEY_PATH"));
    let tls = RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .unwrap_or_else(|error| {
            eprintln!("failed to load the TLS certificate ({cert_path}, {key_path}): {error}");
            std::process::exit(1)
        });

    println!("Listening on https://{addr}");
    if let Err(error) = axum_server::bind_rustls(addr, tls)
        .serve(app.into_make_service())
        .await
    {
        eprintln!("server stopped unexpectedly: {error}");
        std::process::exit(1);
    }
}
