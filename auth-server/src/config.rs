use std::sync::LazyLock;

use tracing::level_filters::LevelFilter;

pub struct Config;

#[fnmock::fakeable]
impl Config {
    pub fn redis_url() -> &'static str {
        &CONFIG.redis_url
    }

    pub fn identity_server_url() -> &'static str {
        &CONFIG.identity_server_url
    }

    pub fn database_url() -> &'static str {
        &CONFIG.database_url
    }

    pub fn app_port() -> u16 {
        CONFIG.app_port
    }

    pub fn access_token_ttl() -> u32 {
        CONFIG.access_token_ttl
    }

    pub fn login_session_ttl() -> u64 {
        CONFIG.login_session_ttl
    }

    pub fn user_session_ttl() -> u64 {
        CONFIG.user_session_ttl
    }

    pub fn par_ttl() -> u64 {
        CONFIG.par_ttl
    }

    pub fn authorization_code_ttl() -> u64 {
        CONFIG.authorization_code_ttl
    }

    pub fn iss() -> &'static str {
        &CONFIG.iss
    }

    pub fn log_level() -> LevelFilter {
        CONFIG.log_level
    }
}

struct ConfigValues {
    redis_url: String,
    identity_server_url: String,
    database_url: String,
    app_port: u16,
    access_token_ttl: u32,
    login_session_ttl: u64,
    user_session_ttl: u64,
    par_ttl: u64,
    authorization_code_ttl: u64,
    iss: String,
    log_level: LevelFilter,
}

static CONFIG: LazyLock<ConfigValues> = LazyLock::new(|| {
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");

    let identity_server_url =
        std::env::var("IDENTITY_SERVER_URL").expect("IDENTITY_SERVER_URL must be set");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let app_port = std::env::var("APP_PORT")
        .expect("APP_PORT must be set")
        .parse()
        .expect("APP_PORT must be a valid port number");

    let access_token_ttl = std::env::var("ACCESS_TOKEN_TTL")
        .expect("ACCESS_TOKEN_TTL must be set")
        .parse()
        .expect("ACCESS_TOKEN_TTL must be a valid duration");

    let login_session_ttl = std::env::var("LOGIN_SESSION_TTL")
        .expect("LOGIN_SESSION_TTL must be set")
        .parse()
        .expect("LOGIN_SESSION_TTL must be a valid duration");

    let user_session_ttl = std::env::var("USER_SESSION_TTL")
        .expect("USER_SESSION_TTL must be set")
        .parse()
        .expect("USER_SESSION_TTL must be a valid duration");

    let par_ttl = std::env::var("PAR_TTL")
        .expect("PAR_TTL must be set")
        .parse()
        .expect("PAR_TTL must be a valid duration");

    let authorization_code_ttl = std::env::var("AUTHORIZATION_CODE_TTL")
        .expect("AUTHORIZATION_CODE_TTL must be set")
        .parse()
        .expect("AUTHORIZATION_CODE_TTL must be a valid duration");

    let iss = std::env::var("ISSUER_IDENTIFIER").expect("ISSUER_IDENTIFIER must be set");

    let log_level =
        crate::logging::parse_level(&std::env::var("LOG_LEVEL").expect("LOG_LEVEL must be set"))
            .expect("LOG_LEVEL must be one of debug, info, warn, error");

    ConfigValues {
        redis_url,
        identity_server_url,
        database_url,
        app_port,
        access_token_ttl,
        login_session_ttl,
        user_session_ttl,
        par_ttl,
        authorization_code_ttl,
        iss,
        log_level,
    }
});
