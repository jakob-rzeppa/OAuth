use std::sync::LazyLock;

use tracing::level_filters::LevelFilter;

pub struct Config {
    database_url: String,
    database_pepper: String,
    temporary_password_length: usize,
    app_port: u16,
    log_level: LevelFilter,
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let database_pepper = std::env::var("DATABASE_PEPPER").expect("DATABASE_PEPPER must be set");

    let temporary_password_length = std::env::var("TEMPORARY_PASSWORD_LENGTH")
        .expect("TEMPORARY_PASSWORD_LENGTH must be set")
        .parse()
        .expect("TEMPORARY_PASSWORD_LENGTH must be a valid number");

    let app_port = std::env::var("APP_PORT")
        .expect("APP_PORT must be set")
        .parse()
        .expect("APP_PORT must be a valid port number");

    let log_level =
        crate::logging::parse_level(&std::env::var("LOG_LEVEL").expect("LOG_LEVEL must be set"))
            .expect("LOG_LEVEL must be one of debug, info, warn, error");

    Config {
        database_url,
        database_pepper,
        temporary_password_length,
        app_port,
        log_level,
    }
});

impl Config {
    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    pub fn database_pepper(&self) -> &str {
        &self.database_pepper
    }

    pub fn temporary_password_length(&self) -> usize {
        self.temporary_password_length
    }

    pub fn app_port(&self) -> u16 {
        self.app_port
    }

    pub fn log_level(&self) -> LevelFilter {
        self.log_level
    }
}
