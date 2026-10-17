/// Base URLs of the running services under test.
///
/// Loaded from the process environment, `.env.local` and `.env`, in that order of precedence.
#[derive(Debug, Clone)]
pub struct Config {
    identity_server_url: String,
    auth_server_url: String,
    spec_provider_url: String,

    // seed
    client_id: String,
    redirect_uri: String,
    scope: String,

    // auth-server's ISSUER_ORIGIN, which is also its issuer identifier
    issuer: String,
}

impl Config {
    /// Loads `.env.local` and `.env` (never overriding variables that are already set), then reads
    /// the service URLs from the environment.
    ///
    /// # Panics
    ///
    /// Panics if one of the variables is not set.
    pub fn load() -> Self {
        dotenv::dotenv().unwrap();

        Self {
            identity_server_url: std::env::var("IDENTITY_SERVER_URL").unwrap(),
            auth_server_url: std::env::var("AUTH_SERVER_URL").unwrap(),
            spec_provider_url: std::env::var("SPEC_PROVIDER_URL").unwrap(),
            client_id: std::env::var("CLIENT_ID").unwrap(),
            redirect_uri: std::env::var("REDIRECT_URI").unwrap(),
            scope: std::env::var("SCOPE").unwrap(),
            issuer: std::env::var("ISSUER_ORIGIN").unwrap(),
        }
    }

    pub fn identity_url(&self, path: &str) -> String {
        join(&self.identity_server_url, path)
    }

    pub fn auth_url(&self, path: &str) -> String {
        join(&self.auth_server_url, path)
    }

    pub fn spec_provider_url(&self, path: &str) -> String {
        join(&self.spec_provider_url, path)
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }
}

/// Joins a base URL and a path with exactly one `/` between them. The path is otherwise left
/// unchanged, so a trailing slash is preserved.
fn join(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}
