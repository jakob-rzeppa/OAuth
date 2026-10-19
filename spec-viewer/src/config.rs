/// The settings read from the environment, once at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub app_port: u16,
    pub spec_provider_url: String,
    pub content_storage_url: String,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} must be set")]
    Missing(&'static str),
    #[error("APP_PORT must be a valid port number, got {0:?}")]
    InvalidPort(String),
    /// The spec-provider is only reachable over https. The content client has no TLS yet.
    #[error("{name} must be an {scheme}:// url, got {url:?}")]
    UnsupportedScheme {
        name: &'static str,
        scheme: &'static str,
        url: String,
    },
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let require = |name: &'static str| lookup(name).ok_or(ConfigError::Missing(name));
        let require_url = |name: &'static str, scheme: &'static str| {
            let url = require(name)?;
            if url
                .to_ascii_lowercase()
                .starts_with(&format!("{scheme}://"))
            {
                Ok(url)
            } else {
                Err(ConfigError::UnsupportedScheme { name, scheme, url })
            }
        };

        let app_port = require("APP_PORT")?;
        Ok(Config {
            app_port: app_port
                .parse()
                .map_err(|_| ConfigError::InvalidPort(app_port))?,
            spec_provider_url: require_url("SPEC_PROVIDER_URL", "https")?,
            content_storage_url: require_url("CONTENT_STORAGE_URL", "http")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        }
    }

    const ALL: [(&str, &str); 3] = [
        ("APP_PORT", "8080"),
        ("SPEC_PROVIDER_URL", "https://provider:50051"),
        ("CONTENT_STORAGE_URL", "http://content:50052"),
    ];

    #[test]
    fn reads_the_settings() {
        assert_eq!(
            Config::from_lookup(lookup(&ALL)),
            Ok(Config {
                app_port: 8080,
                spec_provider_url: "https://provider:50051".into(),
                content_storage_url: "http://content:50052".into(),
            })
        );
    }

    #[test]
    fn names_the_missing_setting() {
        for missing in ["APP_PORT", "SPEC_PROVIDER_URL", "CONTENT_STORAGE_URL"] {
            let vars: Vec<_> = ALL.into_iter().filter(|(key, _)| *key != missing).collect();

            assert_eq!(
                Config::from_lookup(lookup(&vars)),
                Err(ConfigError::Missing(missing))
            );
        }
    }

    #[test]
    fn rejects_urls_with_the_wrong_scheme() {
        for (name, scheme, url) in [
            ("SPEC_PROVIDER_URL", "https", "http://provider:50051"),
            ("SPEC_PROVIDER_URL", "https", "provider:50051"),
            ("CONTENT_STORAGE_URL", "http", "https://content"),
            ("CONTENT_STORAGE_URL", "http", "content:50052"),
            ("CONTENT_STORAGE_URL", "http", "ftp://content"),
        ] {
            let vars: Vec<_> = ALL
                .into_iter()
                .map(|var| if var.0 == name { (name, url) } else { var })
                .collect();

            assert_eq!(
                Config::from_lookup(lookup(&vars)),
                Err(ConfigError::UnsupportedScheme {
                    name,
                    scheme,
                    url: url.into()
                })
            );
        }
    }

    #[test]
    fn accepts_the_scheme_in_any_case() {
        let vars = [ALL[0], ("SPEC_PROVIDER_URL", "HTTPS://provider"), ALL[2]];

        assert!(Config::from_lookup(lookup(&vars)).is_ok());
    }

    #[test]
    fn rejects_an_invalid_port() {
        let vars = [("APP_PORT", "http"), ALL[1], ALL[2]];

        assert_eq!(
            Config::from_lookup(lookup(&vars)),
            Err(ConfigError::InvalidPort("http".into()))
        );
    }
}
