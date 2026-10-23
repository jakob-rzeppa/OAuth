use std::sync::LazyLock;

/// Initializes the configuration by reading environment variables. This function should be called at the start of the application.
pub fn init() {
    LazyLock::force(&CONFIG);
}

pub fn auth_server_url() -> &'static str {
    &CONFIG.auth_server_url
}

struct Config {
    auth_server_url: String,
}

static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let auth_server_url = std::env::var("AUTH_SERVER_URL").expect("AUTH_SERVER_URL must be set");
    Config { auth_server_url }
});
