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

    // OCID params, optional
    /// Only set if the `openid` scope was granted.
    openid: Option<OpenIdParams>,
}

/// The parameters of an OpenID Connect request, which are only part of the code if the `openid` scope was granted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct OpenIdParams {
    nonce: String,
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
        let openid = nonce
            .filter(|_| has_openid_scope(&scope))
            .map(|nonce| OpenIdParams { nonce });

        Self {
            code,
            client_id,
            scope,
            sub,
            code_challenge,
            code_challenge_method,
            openid,
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
        self.openid.as_ref().map(|openid| openid.nonce.as_str())
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
    fn codes_stored_without_a_nonce_still_deserialize() {
        let mut value = serde_json::to_value(make_code(None)).unwrap();
        value.as_object_mut().unwrap().remove("openid");

        let restored: AuthorizationCode = serde_json::from_value(value).unwrap();

        assert_eq!(restored.nonce(), None);
    }
}
