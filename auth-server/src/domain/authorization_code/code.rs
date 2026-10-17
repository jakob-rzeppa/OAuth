use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::authorization_code::openid::has_openid_scope;

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

    // OIDC params
    /// Only set if the `openid` scope was granted, else ignored.
    nonce: Option<String>,
    /// When the user logged in, which becomes the `auth_time` claim of the ID token.
    auth_time: DateTime<Utc>,
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
        auth_time: DateTime<Utc>,
    ) -> Self {
        let nonce = nonce.filter(|_| has_openid_scope(&scope));

        Self {
            code,
            client_id,
            scope,
            sub,
            code_challenge,
            code_challenge_method,
            nonce,
            auth_time,
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

    #[allow(dead_code)] // read by the ID token grant, which is not implemented yet
    pub fn auth_time(&self) -> &DateTime<Utc> {
        &self.auth_time
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_code_with_scope(scope: &str, nonce: Option<String>) -> AuthorizationCode {
        AuthorizationCode::new(
            "the-code".to_string(),
            Uuid::new_v4(),
            scope.to_string(),
            Uuid::new_v4(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string(),
            "S256".to_string(),
            nonce,
            Utc::now(),
        )
    }

    fn make_code(nonce: Option<String>) -> AuthorizationCode {
        make_code_with_scope("openid", nonce)
    }

    #[test]
    fn keeps_the_nonce_when_the_openid_scope_is_granted() {
        let code = make_code_with_scope("read openid", Some("some-nonce".to_string()));

        assert_eq!(code.nonce(), Some("some-nonce"));
    }

    #[test]
    fn drops_the_nonce_when_the_openid_scope_is_not_granted() {
        let code = make_code_with_scope("read write", Some("some-nonce".to_string()));

        assert_eq!(code.nonce(), None);
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
    fn keeps_the_auth_time() {
        let auth_time = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

        let code = AuthorizationCode::new(
            "the-code".to_string(),
            Uuid::new_v4(),
            "openid".to_string(),
            Uuid::new_v4(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string(),
            "S256".to_string(),
            Some("some-nonce".to_string()),
            auth_time,
        );

        assert_eq!(code.auth_time(), &auth_time);
    }

    #[test]
    fn codes_stored_without_a_nonce_still_deserialize() {
        let mut value = serde_json::to_value(make_code(None)).unwrap();
        value.as_object_mut().unwrap().remove("nonce");

        let restored: AuthorizationCode = serde_json::from_value(value).unwrap();

        assert_eq!(restored.nonce(), None);
    }
}
