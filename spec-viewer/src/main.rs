use std::{future::Future, net::SocketAddr, sync::Arc};

use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing_subscriber::EnvFilter;

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
    init_tracing();

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
    let listener = TcpListener::bind(addr)
        .await
        .unwrap_or_else(|error| exit_with(format_args!("failed to bind {addr}: {error}")));

    tracing::info!(%addr, "spec-viewer listening");
    if let Err(error) = serve(listener, app(state), shutdown_signal()).await {
        exit_with(format_args!("server stopped unexpectedly: {error}"));
    }
    tracing::info!("spec-viewer stopped");
}

/// Logs to stdout. `RUST_LOG` sets what is logged, like `debug` or `spec_viewer=debug`; the
/// default is `info`.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

/// The health check stays out of the request log, so the container's checks don't flood it.
///
/// A failed request is logged once, with its status. The handlers log what went wrong.
fn app(state: AppState) -> Router {
    let request_log = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO))
        .on_failure(());

    Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(web::router().layer(request_log))
        .with_state(state)
}

/// Serves until `shutdown` resolves, then stops accepting connections and waits for the
/// requests in flight.
async fn serve(
    listener: TcpListener,
    app: Router,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
}

/// Resolves on Ctrl+C or SIGTERM (sent by `docker stop`).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to listen for SIGTERM")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("signal received, shutting down gracefully");
}

fn exit_with(error: impl std::fmt::Display) -> ! {
    tracing::error!("{error}");
    std::process::exit(1)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::application::contract::{
        content_gateway::MockContentGateway, spec_gateway::MockSpecGateway,
    };

    fn state() -> AppState {
        let spec_gateway = Arc::new(MockSpecGateway::new());
        AppState::new(
            SpecViewService::new(spec_gateway.clone(), Arc::new(MockContentGateway::new())),
            SpecListService::new(spec_gateway),
        )
    }

    async fn start() -> (
        String,
        tokio::sync::oneshot::Sender<()>,
        tokio::task::JoinHandle<std::io::Result<()>>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(serve(listener, app(state()), async {
            let _ = stopped.await;
        }));
        (url, stop, server)
    }

    #[tokio::test]
    async fn answers_the_health_check() {
        let (url, _stop, _server) = start().await;

        let response = reqwest::get(format!("{url}/health")).await.unwrap();

        assert!(response.status().is_success());
        assert_eq!(response.text().await.unwrap(), "ok");
    }

    #[tokio::test]
    async fn stops_serving_when_the_shutdown_signal_resolves() {
        let (url, stop, server) = start().await;
        assert!(reqwest::get(format!("{url}/health")).await.is_ok());

        stop.send(()).unwrap();

        let result = tokio::time::timeout(Duration::from_secs(5), server).await;
        assert!(matches!(result, Ok(Ok(Ok(())))), "{result:?}");
        assert!(reqwest::get(format!("{url}/health")).await.is_err());
    }
}
