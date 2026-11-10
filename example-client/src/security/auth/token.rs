use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

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

    /// The ID token, present when the `openid` scope was granted.
    pub fn id_token(&self) -> Option<&str> {
        self.id_token.as_deref()
    }

    /// The `iss` claim of the ID token, `None` if there is no ID token or it carries no `iss`.
    ///
    /// The signature isn't verified: the token came straight from the auth-server over TLS, which
    /// OpenID Connect Core 1.0 section 3.1.3.7 allows in place of the signature check. The `iss`
    /// still has to be compared.
    pub fn id_token_issuer(&self) -> Option<String> {
        let claims = jwt_claims(self.id_token.as_deref()?)?;
        claims.get("iss")?.as_str().map(str::to_string)
    }
}

/// The decoded JOSE header of a compact JWT, without verifying anything.
pub fn jwt_header(jwt: &str) -> Option<serde_json::Map<String, serde_json::Value>> {
    jwt_part(jwt, 0)
}

/// The decoded claims of a compact JWT, without verifying anything.
pub fn jwt_claims(jwt: &str) -> Option<serde_json::Map<String, serde_json::Value>> {
    jwt_part(jwt, 1)
}

fn jwt_part(jwt: &str, index: usize) -> Option<serde_json::Map<String, serde_json::Value>> {
    let part = jwt.split('.').nth(index)?;
    let bytes = URL_SAFE_NO_PAD.decode(part).ok()?;
    serde_json::from_slice(&bytes).ok()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn response_with(id_token: Option<String>) -> TokenResponse {
        TokenResponse {
            access_token: "at".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 60,
            scope: "openid".to_string(),
            id_token,
        }
    }

    fn jwt(claims: &str) -> String {
        format!("e30.{}.sig", URL_SAFE_NO_PAD.encode(claims))
    }

    #[test]
    fn reads_the_iss_claim_of_the_id_token() {
        let response = response_with(Some(jwt(r#"{"iss":"https://auth.example"}"#)));
        assert_eq!(response.id_token_issuer().as_deref(), Some("https://auth.example"));
    }

    #[test]
    fn has_no_issuer_without_an_id_token_or_an_iss_claim() {
        assert_eq!(response_with(None).id_token_issuer(), None);
        assert_eq!(response_with(Some(jwt(r#"{"sub":"x"}"#))).id_token_issuer(), None);
        assert_eq!(response_with(Some("not-a-jwt".to_string())).id_token_issuer(), None);
    }
}
