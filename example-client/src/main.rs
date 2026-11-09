use std::net::SocketAddr;

use askama::Template;
use axum::{
    Router,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use axum_server::tls_rustls::RustlsConfig;

mod security;
mod config;
mod app_state;

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

#[tokio::main]
async fn main() {
    config::init();

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
