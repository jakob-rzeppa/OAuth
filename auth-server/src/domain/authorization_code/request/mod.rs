use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod validate;

/// The authorization request.
///
/// We store every value in a `Option` because the request may be incomplete or malformed.
/// This allows us to validate the params in order to provide more specific error messages to the client
/// and differentiate between errors that can be redirected to the client and those that cannot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthorizationRequest {
    client_id: Uuid,

    redirect_uri: String,
    response_type: String,
    scope: String,

    state: String,

    // PKCE params
    code_challenge: String,
    code_challenge_method: String,

    // OCID params, optional
    nonce: Option<String>,
}

impl AuthorizationRequest {
    pub fn new(
        client_id: Uuid,
        redirect_uri: String,
        response_type: String,
        scope: String,
        state: String,
        code_challenge: String,
        code_challenge_method: String,
        nonce: Option<String>,
    ) -> Self {
        Self {
            client_id,
            redirect_uri,
            response_type,
            scope,
            state,
            code_challenge,
            code_challenge_method,
            nonce,
        }
    }

    pub fn client_id(&self) -> &Uuid {
        &self.client_id
    }

    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    pub fn response_type(&self) -> &str {
        &self.response_type
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    pub fn state(&self) -> &str {
        &self.state
    }

    pub fn code_challenge(&self) -> &str {
        &self.code_challenge
    }

    pub fn code_challenge_method(&self) -> &str {
        &self.code_challenge_method
    }

    #[cfg(test)]
    pub fn nonce(&self) -> Option<&str> {
        self.nonce.as_deref()
    }
}
