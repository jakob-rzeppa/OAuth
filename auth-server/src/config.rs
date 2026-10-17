use std::sync::LazyLock;

use tracing::level_filters::LevelFilter;

pub struct Config;

#[fnmock::fakeable]
impl Config {
    pub fn init() {
        LazyLock::force(&CONFIG);
    }

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

    pub fn user_session_ttl() -> u64 {
        CONFIG.user_session_ttl
    }

    pub fn par_ttl() -> u64 {
        CONFIG.par_ttl
    }

    pub fn authorization_code_ttl() -> u64 {
        CONFIG.authorization_code_ttl
    }

    pub fn id_token_ttl() -> u64 {
        CONFIG.id_token_ttl
    }

    /// The origin (`scheme://host[:port]`) browsers reach the auth-server under. Also the issuer
    /// identifier (`iss`).
    pub fn iss() -> &'static str {
        &CONFIG.iss
    }

    /// Path of the PEM file with the RSA private key ID tokens are signed with.
    pub fn signing_key_path() -> &'static str {
        &CONFIG.signing_key_path
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
    user_session_ttl: u64,
    par_ttl: u64,
    authorization_code_ttl: u64,
    id_token_ttl: u64,
    iss: String,
    signing_key_path: String,
    log_level: LevelFilter,
}

/// Parses `value` as a bare origin (`scheme://host[:port]`) and returns it in the form
/// browsers send in the `Origin` header.
fn parse_iss(value: &str) -> Result<String, String> {
    let url = url::Url::parse(value).map_err(|error| error.to_string())?;

    if !url.has_host() || url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        return Err("expected scheme://host[:port] without a path, query or fragment".into());
    }

    Ok(url.origin().ascii_serialization())
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

    let id_token_ttl = std::env::var("ID_TOKEN_TTL")
        .expect("ID_TOKEN_TTL must be set")
        .parse()
        .expect("ID_TOKEN_TTL must be a valid duration");

    let iss =
        parse_iss(&std::env::var("ISSUER_ORIGIN").expect("ISSUER_ORIGIN must be set"))
            .expect("ISSUER_ORIGIN must be an origin like https://auth.example.com");

    let signing_key_path =
        std::env::var("SIGNING_KEY_PATH").expect("SIGNING_KEY_PATH must be set");

    let log_level =
        crate::logging::parse_level(&std::env::var("LOG_LEVEL").expect("LOG_LEVEL must be set"))
            .expect("LOG_LEVEL must be one of debug, info, warn, error");

    ConfigValues {
        redis_url,
        identity_server_url,
        database_url,
        app_port,
        access_token_ttl,
        user_session_ttl,
        par_ttl,
        authorization_code_ttl,
        id_token_ttl,
        iss,
        signing_key_path,
        log_level,
    }
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_iss_origins_into_the_form_browsers_send() {
        assert_eq!(
            parse_iss("https://Auth.Example.com/").unwrap(),
            "https://auth.example.com"
        );
        assert_eq!(
            parse_iss("http://127.0.0.1:8081").unwrap(),
            "http://127.0.0.1:8081"
        );
        // The default port is dropped, as browsers do.
        assert_eq!(
            parse_iss("https://auth.example.com:443").unwrap(),
            "https://auth.example.com"
        );
    }

    #[test]
    fn rejects_values_that_are_not_a_bare_origin() {
        for value in [
            "a-identifier-you-should-change",
            "https://auth.example.com/path",
            "https://auth.example.com/?q=1",
            "https://auth.example.com/#fragment",
            "mailto:someone@example.com",
        ] {
            assert!(parse_iss(value).is_err(), "{value}");
        }
    }
}
