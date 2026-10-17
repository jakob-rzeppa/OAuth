use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    config::Config, domain::authorization_code::code::AuthorizationCode, error::InternalError,
    jwks::signing_key,
};

/// The claims of an OpenID Connect ID token (OpenID Connect Core 1.0, section 2).
#[derive(Debug, Serialize)]
struct IdTokenClaims<'a> {
    /// Issuer Identifier. Matches the `iss` of the authorization response and introspection.
    iss: &'a str,
    /// Subject Identifier. The user's id, which is never reassigned.
    sub: Uuid,
    /// Audience. The `client_id` of the client the token was issued to.
    aud: Uuid,
    #[serde(with = "chrono::serde::ts_seconds")]
    exp: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    iat: DateTime<Utc>,
    /// When the user logged in.
    #[serde(with = "chrono::serde::ts_seconds")]
    auth_time: DateTime<Utc>,
    /// Echoes the `nonce` of the authorization request. Only present if the client sent one.
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<&'a str>,
}

/// Generates the signed ID token for a redeemed authorization code. Only call it if the `openid`
/// scope was granted.
#[fnmock::mockable]
pub(super) fn generate_id_token(
    authorization_code: &AuthorizationCode,
) -> Result<String, InternalError> {
    let iat = Utc::now();
    let exp = iat + chrono::Duration::seconds(Config::id_token_ttl() as i64);

    let claims = IdTokenClaims {
        iss: Config::iss(),
        sub: *authorization_code.sub(),
        aud: *authorization_code.client_id(),
        exp,
        iat,
        auth_time: *authorization_code.auth_time(),
        nonce: authorization_code.nonce(),
    };

    Ok(signing_key().sign(&claims)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jwks::{TEST_SIGNING_KEY, signing_key_fake};
    use jsonwebtoken::{Algorithm, DecodingKey, Validation};

    const ISSUER: &str = "https://auth.example.com";
    const ID_TOKEN_TTL_SECONDS: u64 = 300;

    fn make_code(scope: &str, nonce: Option<&str>, auth_time: DateTime<Utc>) -> AuthorizationCode {
        AuthorizationCode::new(
            "the-code".to_string(),
            Uuid::new_v4(),
            scope.to_string(),
            Uuid::new_v4(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string(),
            "S256".to_string(),
            nonce.map(str::to_string),
            auth_time,
        )
    }

    fn setup() {
        signing_key_fake().setup(|| &TEST_SIGNING_KEY);
        Config::iss_fake().setup(|| ISSUER);
        Config::id_token_ttl_fake().setup(|| ID_TOKEN_TTL_SECONDS);
    }

    /// Verifies the token the way a client does: signature against the published key, `iss`, `aud`
    /// and `exp`. Returns the claims.
    fn verify(token: &str, client_id: &Uuid) -> serde_json::Value {
        let decoding_key = DecodingKey::from_jwk(&TEST_SIGNING_KEY.jwks().keys[0]).unwrap();
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[ISSUER]);
        validation.set_audience(&[client_id.to_string()]);
        validation.set_required_spec_claims(&["iss", "sub", "aud", "exp", "iat"]);

        jsonwebtoken::decode::<serde_json::Value>(token, &decoding_key, &validation)
            .unwrap()
            .claims
    }

    #[test]
    fn signs_the_claims_of_the_authorization_code() {
        setup();
        let auth_time = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        let code = make_code("openid read", Some("some-nonce"), auth_time);

        let before = Utc::now().timestamp();
        let token = generate_id_token(&code).unwrap();
        let after = Utc::now().timestamp();

        let claims = verify(&token, code.client_id());
        assert_eq!(claims["iss"], ISSUER);
        assert_eq!(claims["sub"], code.sub().to_string());
        assert_eq!(claims["aud"], code.client_id().to_string());
        assert_eq!(claims["auth_time"], 1_700_000_000);
        assert_eq!(claims["nonce"], "some-nonce");

        let iat = claims["iat"].as_i64().unwrap();
        assert!((before..=after).contains(&iat));
        assert_eq!(
            claims["exp"].as_i64().unwrap(),
            iat + ID_TOKEN_TTL_SECONDS as i64
        );
    }

    #[test]
    fn carries_the_kid_of_the_signing_key() {
        setup();
        let code = make_code("openid", Some("some-nonce"), Utc::now());

        let token = generate_id_token(&code).unwrap();

        let header = jsonwebtoken::decode_header(&token).unwrap();
        assert_eq!(header.alg, Algorithm::RS256);
        assert_eq!(header.kid, TEST_SIGNING_KEY.jwks().keys[0].common.key_id);
    }

    #[test]
    fn leaves_out_the_nonce_when_the_code_has_none() {
        setup();
        let code = make_code("openid", None, Utc::now());

        let token = generate_id_token(&code).unwrap();

        let claims = verify(&token, code.client_id());
        assert!(claims.get("nonce").is_none(), "{claims}");
    }
}
