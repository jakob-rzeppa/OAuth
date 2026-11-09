use std::sync::LazyLock;

struct Config {
    auth_server_url: String,
    client_id: String,
    tls_cert_path: String,
    tls_key_path: String,
    home_url: String,
}

static CONFIG: LazyLock<Config> = LazyLock::new(|| Config {
    auth_server_url: std::env::var("AUTH_SERVER_URL").unwrap_or_else(|_| {
        panic!("AUTH_SERVER_URL must be set");
    }),
    client_id: std::env::var("CLIENT_ID").unwrap_or_else(|_| {
        panic!("CLIENT_ID must be set");
    }),
    tls_cert_path: std::env::var("TLS_CERT_PATH").unwrap_or_else(|_| {
        panic!("TLS_CERT_PATH must be set");
    }),
    tls_key_path: std::env::var("TLS_KEY_PATH").unwrap_or_else(|_| {
        panic!("TLS_KEY_PATH must be set");
    }),
    home_url: std::env::var("HOME_URL").unwrap_or_else(|_| {
        panic!("HOME_URL must be set");
    }),
});

pub fn init() {
    LazyLock::force(&CONFIG);
}

pub fn auth_server_url() -> String {
    CONFIG.auth_server_url.clone()
}

pub fn client_id() -> String {
    CONFIG.client_id.clone()
}

pub fn tls_cert_path() -> String {
    CONFIG.tls_cert_path.clone()
}

pub fn tls_key_path() -> String {
    CONFIG.tls_key_path.clone()
}

pub fn home_url() -> String {
    CONFIG.home_url.clone()
}
