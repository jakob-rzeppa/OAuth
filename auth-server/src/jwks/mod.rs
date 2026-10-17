use std::sync::LazyLock;

use axum::{
    Json, Router,
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
    routing::get,
};
use jsonwebtoken::{
    Algorithm, EncodingKey, Header,
    jwk::{Jwk, JwkSet, PublicKeyUse, ThumbprintHash},
};
use serde::Serialize;

use crate::config::Config;

/// The algorithm the ID tokens are signed with.
const SIGNING_ALGORITHM: Algorithm = Algorithm::RS256;

/// How long clients may cache the key set. Clients refetch on an unknown `kid` regardless.
const CACHE_CONTROL: &str = "public, max-age=3600";

/// The key ID tokens are signed with, loaded once from the PEM file at `SIGNING_KEY_PATH`.
static SIGNING_KEY: LazyLock<SigningKey> = LazyLock::new(|| {
    let pem = std::fs::read(Config::signing_key_path()).unwrap_or_else(|error| {
        panic!(
            "SIGNING_KEY_PATH ({}) must be a readable file: {error}",
            Config::signing_key_path()
        )
    });

    SigningKey::from_pem(&pem).unwrap_or_else(|error| {
        panic!("SIGNING_KEY_PATH must contain an RSA private key in PEM format: {error}")
    })
});

/// Loads the signing key now, so a missing or invalid key stops the server at startup instead of
/// failing the first request.
pub fn init() {
    LazyLock::force(&SIGNING_KEY);
}

#[fnmock::fakeable]
pub fn signing_key() -> &'static SigningKey {
    &SIGNING_KEY
}

/// The private key ID tokens are signed with, together with the public key set published at
/// `/jwks.json`.
pub struct SigningKey {
    encoding_key: EncodingKey,
    kid: String,
    jwks: JwkSet,
}

impl SigningKey {
    /// Builds the key from an RSA private key in PEM format. Only the public parameters (`n` and
    /// `e`) end up in the key set. The `kid` is the RFC 7638 thumbprint of the public key, so it
    /// changes exactly when the key does.
    pub fn from_pem(pem: &[u8]) -> jsonwebtoken::errors::Result<Self> {
        let encoding_key = EncodingKey::from_rsa_pem(pem)?;

        let mut jwk = Jwk::from_encoding_key(&encoding_key, SIGNING_ALGORITHM)?;
        let kid = jwk.thumbprint(ThumbprintHash::SHA256)?;
        jwk.common.public_key_use = Some(PublicKeyUse::Signature);
        jwk.common.key_id = Some(kid.clone());

        Ok(Self {
            encoding_key,
            kid,
            jwks: JwkSet { keys: vec![jwk] },
        })
    }

    pub fn jwks(&self) -> &JwkSet {
        &self.jwks
    }

    /// Signs `claims` into a compact JWS, with the `kid` in the header so clients can pick the
    /// matching key from the key set.
    pub fn sign(&self, claims: &impl Serialize) -> jsonwebtoken::errors::Result<String> {
        let mut header = Header::new(SIGNING_ALGORITHM);
        header.kid = Some(self.kid.clone());

        jsonwebtoken::encode(&header, claims, &self.encoding_key)
    }
}

/// A key built from `test_key.pem`, for tests that sign without `SIGNING_KEY_PATH`.
#[cfg(test)]
pub static TEST_SIGNING_KEY: LazyLock<SigningKey> =
    LazyLock::new(|| SigningKey::from_pem(include_bytes!("test_key.pem")).unwrap());

pub fn router() -> Router {
    Router::new().route("/jwks.json", get(jwks_endpoint))
}

pub async fn jwks_endpoint() -> Response {
    (
        [(header::CACHE_CONTROL, HeaderValue::from_static(CACHE_CONTROL))],
        Json(signing_key().jwks()),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway 2048-bit key, generated for this test.
    const TEST_KEY: &str = include_str!("test_key.pem");

    fn jwks_from_pem(pem: &[u8]) -> jsonwebtoken::errors::Result<JwkSet> {
        SigningKey::from_pem(pem).map(|key| key.jwks)
    }

    #[test]
    fn publishes_the_public_rsa_parameters_of_the_signing_key() {
        let jwks = serde_json::to_value(jwks_from_pem(TEST_KEY.as_bytes()).unwrap()).unwrap();

        let keys = jwks["keys"].as_array().unwrap();
        assert_eq!(keys.len(), 1);
        let key = &keys[0];
        assert_eq!(key["kty"], "RSA");
        assert_eq!(key["use"], "sig");
        assert_eq!(key["alg"], "RS256");
        assert_eq!(key["e"], "AQAB");
        assert!(key["n"].as_str().unwrap().len() > 300);
        assert!(!key["kid"].as_str().unwrap().is_empty());
    }

    #[test]
    fn never_publishes_private_key_parameters() {
        let jwks = serde_json::to_value(jwks_from_pem(TEST_KEY.as_bytes()).unwrap()).unwrap();

        let key = jwks["keys"][0].as_object().unwrap();
        for private in ["d", "p", "q", "dp", "dq", "qi"] {
            assert!(!key.contains_key(private), "{private}");
        }
    }

    #[test]
    fn the_key_id_is_stable_for_the_same_key() {
        let first = jwks_from_pem(TEST_KEY.as_bytes()).unwrap();
        let second = jwks_from_pem(TEST_KEY.as_bytes()).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn rejects_something_that_is_not_an_rsa_pem() {
        assert!(jwks_from_pem(b"not a key").is_err());
    }

    #[test]
    fn signs_with_the_published_key_and_its_kid() {
        let key = SigningKey::from_pem(TEST_KEY.as_bytes()).unwrap();

        let token = key.sign(&serde_json::json!({ "sub": "someone" })).unwrap();

        let header = jsonwebtoken::decode_header(&token).unwrap();
        assert_eq!(header.alg, Algorithm::RS256);
        assert_eq!(header.kid.as_deref(), Some(key.kid.as_str()));
        let decoding_key = jsonwebtoken::DecodingKey::from_jwk(&key.jwks.keys[0]).unwrap();
        let mut validation = jsonwebtoken::Validation::new(Algorithm::RS256);
        validation.required_spec_claims.clear();
        validation.validate_exp = false;
        let claims =
            jsonwebtoken::decode::<serde_json::Value>(&token, &decoding_key, &validation).unwrap();
        assert_eq!(claims.claims["sub"], "someone");
    }

    #[tokio::test]
    async fn the_key_set_is_cacheable_json() {
        signing_key_fake().setup(|| &TEST_SIGNING_KEY);

        let response = jwks_endpoint().await;

        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "public, max-age=3600"
        );
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }
}
