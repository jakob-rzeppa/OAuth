use std::sync::LazyLock;

pub struct Config;

impl Config {
    pub fn app_port() -> u16 {
        CONFIG.app_port
    }

    pub fn spec_provider_url() -> &'static str {
        &CONFIG.spec_provider_url
    }
}

struct ConfigValues {
    app_port: u16,
    spec_provider_url: String,
}

static CONFIG: LazyLock<ConfigValues> = LazyLock::new(|| {
    let app_port = std::env::var("APP_PORT")
        .expect("APP_PORT must be set")
        .parse()
        .expect("APP_PORT must be a valid port number");

    let spec_provider_url =
        std::env::var("SPEC_PROVIDER_URL").expect("SPEC_PROVIDER_URL must be set");

    ConfigValues {
        app_port,
        spec_provider_url,
    }
});
