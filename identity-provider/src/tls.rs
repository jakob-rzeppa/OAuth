use axum_server::tls_rustls::RustlsConfig;

/// Loads the PEM certificate chain and private key at `TLS_CERT_PATH` and `TLS_KEY_PATH`.
///
/// The server only answers over https, so it exits when they are not set or can't be read.
pub async fn load() -> RustlsConfig {
    let require = |name: &str| {
        std::env::var(name).unwrap_or_else(|_| {
            tracing::error!("{name} must be set");
            std::process::exit(1)
        })
    };
    let (cert_path, key_path) = (require("TLS_CERT_PATH"), require("TLS_KEY_PATH"));

    RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .unwrap_or_else(|error| {
            tracing::error!(%cert_path, %key_path, ?error, "failed to load the TLS certificate");
            std::process::exit(1)
        })
}
