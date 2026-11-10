use crate::config;

/// The query the auth-server redirects the browser back with (RFC 6749 section 4.1.2 and
/// 4.1.2.1, `iss` from RFC 9207). Every field is optional, so a plain visit to `/` parses too.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CallbackQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
}

impl CallbackQuery {
    /// Whether this is a callback from the auth-server rather than a plain visit.
    pub fn is_callback(&self) -> bool {
        self.code.is_some() || self.error.is_some()
    }
}

/// A successful authorization response, with the state and issuer already checked.
#[derive(serde::Serialize)]
pub struct AuthorizationResponse {
    pub code: String,
    pub state: String,
    pub iss: String,
}

/// Where the browser is sent to authorize the pushed request (RFC 9126 section 4).
pub fn authorization_url(request_uri: &str) -> String {
    let query = serde_urlencoded::to_string([
        ("client_id", config::client_id()),
        ("request_uri", request_uri.to_string()),
    ])
    .expect("two string pairs always encode");
    format!("{}/authorize?{query}", config::auth_server_url())
}
