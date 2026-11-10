use crate::{
    config,
    security::auth::{code_verifier::CodeVerifier, error::AuthError, nonce::Nonce, state::State},
};

/// The scopes the example-client asks for.
pub const SCOPE: &str = "openid profile";

/// The pushed authorization request (RFC 9126), sent as a form to `POST /par`.
#[derive(serde::Serialize)]
pub struct ParRequest {
    client_id: String,
    redirect_uri: String,
    response_type: String,
    scope: String,
    state: String,
    code_challenge: String,
    code_challenge_method: String,
    nonce: String,
}

impl ParRequest {
    pub fn new(state: &State, nonce: &Nonce, code_verifier: &CodeVerifier) -> Self {
        Self {
            client_id: config::client_id(),
            redirect_uri: config::home_url(),
            response_type: "code".to_string(),
            scope: SCOPE.to_string(),
            state: state.value().to_string(),
            code_challenge: code_verifier.code_challenge(),
            code_challenge_method: code_verifier.code_challenge_method().to_string(),
            nonce: nonce.value().to_string(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ParResponse {
    request_uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_in: Option<u64>,
}

impl ParResponse {
    pub fn request_uri(&self) -> &str {
        &self.request_uri
    }
}

pub fn par_url() -> String {
    config::auth_server_url() + "/par"
}

pub async fn register_par(request: &ParRequest) -> Result<ParResponse, AuthError> {
    let client = reqwest::Client::new();

    let response = client.post(par_url()).form(request).send().await?;
    let response: ParResponse = AuthError::check(response).await?.json().await?;

    Ok(response)
}
