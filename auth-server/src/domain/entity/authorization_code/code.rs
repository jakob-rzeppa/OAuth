use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The authorization code entity, derived from the authorization request on validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthorizationCode {
    code: String,
    client_id: Uuid,

    scope: String,

    sub: Uuid,

    // PKCE params
    code_challenge: String,
    code_challenge_method: String,

    // OCID params, optional
    nonce: Option<String>,
}

impl AuthorizationCode {
    pub fn new(
        code: String,
        client_id: Uuid,
        scope: String,
        sub: Uuid,
        code_challenge: String,
        code_challenge_method: String,
        nonce: Option<String>,
    ) -> Self {
        Self {
            code,
            client_id,
            scope,
            sub,
            code_challenge,
            code_challenge_method,
            nonce,
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn client_id(&self) -> &Uuid {
        &self.client_id
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    pub fn sub(&self) -> &Uuid {
        &self.sub
    }

    pub fn code_challenge(&self) -> &str {
        &self.code_challenge
    }

    pub fn code_challenge_method(&self) -> &str {
        &self.code_challenge_method
    }

    #[allow(dead_code)] // read by the ID token grant, which is not implemented yet
    pub fn nonce(&self) -> Option<&str> {
        self.nonce.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_code(nonce: Option<String>) -> AuthorizationCode {
        AuthorizationCode::new(
            "the-code".to_string(),
            Uuid::new_v4(),
            "openid".to_string(),
            Uuid::new_v4(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string(),
            "S256".to_string(),
            nonce,
        )
    }

    #[test]
    fn nonce_survives_serialization() {
        let code = make_code(Some("some-nonce".to_string()));

        let json = serde_json::to_string(&code).unwrap();
        let restored: AuthorizationCode = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.nonce(), Some("some-nonce"));
        assert_eq!(restored, code);
    }

    #[test]
    fn codes_stored_without_a_nonce_still_deserialize() {
        let mut value = serde_json::to_value(make_code(None)).unwrap();
        value.as_object_mut().unwrap().remove("nonce");

        let restored: AuthorizationCode = serde_json::from_value(value).unwrap();

        assert_eq!(restored.nonce(), None);
    }
}
