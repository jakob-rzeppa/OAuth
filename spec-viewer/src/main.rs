use std::{future::Future, net::SocketAddr, sync::Arc, time::Duration};

use axum::{Router, routing::get};
use axum_server::{Handle, tls_rustls::RustlsConfig};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing_subscriber::EnvFilter;

use crate::{
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
    let state = AppState::new(spec_gateway, content_gateway);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.app_port));
    let listener = std::net::TcpListener::bind(addr)
        .unwrap_or_else(|error| exit_with(format_args!("failed to bind {addr}: {error}")));
    let tls = load_tls().await;

    tracing::info!(%addr, "spec-viewer listening (https)");
    if let Err(error) = serve(listener, tls, app(state), shutdown_signal()).await {
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

/// Loads the PEM certificate chain and private key at `TLS_CERT_PATH` and `TLS_KEY_PATH`.
///
/// The server only answers over https, so it exits when they are not set or can't be read.
async fn load_tls() -> RustlsConfig {
    let require = |name: &str| {
        std::env::var(name).unwrap_or_else(|_| exit_with(format_args!("{name} must be set")))
    };
    let (cert_path, key_path) = (require("TLS_CERT_PATH"), require("TLS_KEY_PATH"));

    RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .unwrap_or_else(|error| {
            exit_with(format_args!(
                "failed to load the TLS certificate {cert_path} and key {key_path}: {error}"
            ))
        })
}

/// How long requests in flight get to finish once `shutdown` resolved.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(10);

/// Serves over https until `shutdown` resolves, then stops accepting connections and waits for
/// the requests in flight.
async fn serve(
    listener: std::net::TcpListener,
    tls: RustlsConfig,
    app: Router,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    listener.set_nonblocking(true)?;
    let handle = Handle::new();
    tokio::spawn({
        let handle = handle.clone();
        async move {
            shutdown.await;
            handle.graceful_shutdown(Some(SHUTDOWN_GRACE));
        }
    });

    axum_server::from_tcp_rustls(listener, tls)?
        .handle(handle)
        .serve(app.into_make_service())
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
        AppState::new(
            Arc::new(MockSpecGateway::new()),
            Arc::new(MockContentGateway::new()),
        )
    }

    /// A server with a throwaway certificate for 127.0.0.1.
    async fn start() -> (
        SocketAddr,
        rustls::pki_types::CertificateDer<'static>,
        tokio::sync::oneshot::Sender<()>,
        tokio::task::JoinHandle<std::io::Result<()>>,
    ) {
        let certified = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()]).unwrap();
        let tls = RustlsConfig::from_pem(
            certified.cert.pem().into_bytes(),
            certified.signing_key.serialize_pem().into_bytes(),
        )
        .await
        .unwrap();

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(serve(listener, tls, app(state()), async {
            let _ = stopped.await;
        }));
        (addr, certified.cert.der().clone(), stop, server)
    }

    const HEALTH_REQUEST: &[u8] =
        b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";

    /// Sends `HEALTH_REQUEST` over TLS, trusting only `cert`, and returns the raw response.
    async fn get_health_over_tls(
        addr: SocketAddr,
        cert: &rustls::pki_types::CertificateDer<'static>,
    ) -> std::io::Result<String> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut roots = rustls::RootCertStore::empty();
        roots.add(cert.clone()).unwrap();
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let connector = tokio_rustls::TlsConnector::from(Arc::new(config));

        let tcp = tokio::net::TcpStream::connect(addr).await?;
        let server_name = rustls::pki_types::ServerName::try_from("127.0.0.1").unwrap();
        let mut stream = connector.connect(server_name, tcp).await?;
        stream.write_all(HEALTH_REQUEST).await?;
        let mut response = String::new();
        stream.read_to_string(&mut response).await?;
        Ok(response)
    }

    #[tokio::test]
    async fn answers_the_health_check_over_https() {
        let (addr, cert, _stop, _server) = start().await;

        let response = get_health_over_tls(addr, &cert).await.unwrap();

        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(response.ends_with("ok"), "{response}");
    }

    #[tokio::test]
    async fn does_not_answer_plain_http() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let (addr, _cert, _stop, _server) = start().await;

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        stream.write_all(HEALTH_REQUEST).await.unwrap();
        let mut response = Vec::new();
        let _ = stream.read_to_end(&mut response).await;

        assert!(!String::from_utf8_lossy(&response).contains("200 OK"));
    }

    #[tokio::test]
    async fn stops_serving_when_the_shutdown_signal_resolves() {
        let (addr, cert, stop, server) = start().await;
        assert!(get_health_over_tls(addr, &cert).await.is_ok());

        stop.send(()).unwrap();

        let result = tokio::time::timeout(Duration::from_secs(5), server).await;
        assert!(matches!(result, Ok(Ok(Ok(())))), "{result:?}");
        assert!(get_health_over_tls(addr, &cert).await.is_err());
    }
}
