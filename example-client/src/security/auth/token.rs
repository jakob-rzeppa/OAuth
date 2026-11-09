use crate::{
    config,
    security::auth::{code_verifier::CodeVerifier, error::AuthError},
};

/// The authorization code grant's token request (RFC 6749 section 4.1.3), sent as a form to
/// `POST /token`, with the PKCE verifier (RFC 7636 section 4.5).
#[derive(serde::Serialize)]
pub struct TokenRequest {
    grant_type: String,
    client_id: String,
    code: String,
    code_verifier: String,
}

impl TokenRequest {
    pub fn new(code: &str, code_verifier: &CodeVerifier) -> Self {
        Self {
            grant_type: "authorization_code".to_string(),
            client_id: config::client_id(),
            code: code.to_string(),
            code_verifier: code_verifier.value().to_string(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id_token: Option<String>,
}

impl TokenResponse {
    pub fn access_token(&self) -> &str {
        &self.access_token
    }
}

pub fn token_url() -> String {
    config::auth_server_url() + "/token"
}

pub async fn request_token(request: &TokenRequest) -> Result<TokenResponse, AuthError> {
    let client = reqwest::Client::new();

    let response = client.post(token_url()).form(request).send().await?;
    let response: TokenResponse = AuthError::check(response).await?.json().await?;

    Ok(response)
}
