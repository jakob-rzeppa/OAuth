use serde::{Deserialize, Serialize};

/// A login session, started when the login page is served and stored under the hash of the
/// session token held by the browser. It is consumed when the login form is submitted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginSession {
    csrf_token: String,
}

impl LoginSession {
    pub fn new(csrf_token: String) -> Self {
        Self { csrf_token }
    }

    pub fn csrf_token(&self) -> &str {
        &self.csrf_token
    }
}
