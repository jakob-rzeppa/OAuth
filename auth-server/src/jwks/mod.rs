use std::sync::LazyLock;

use axum::{
    Json, Router,
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
    routing::get,
};
use jsonwebtoken::{
    Algorithm, EncodingKey,
    jwk::{Jwk, JwkSet, PublicKeyUse, ThumbprintHash},
};

use crate::config::Config;

/// The algorithm the ID tokens are signed with.
const SIGNING_ALGORITHM: Algorithm = Algorithm::RS256;

/// How long clients may cache the key set. Clients refetch on an unknown `kid` regardless.
const CACHE_CONTROL: &str = "public, max-age=3600";

/// The public half of the signing key, built once from the PEM file at `SIGNING_KEY_PATH`.
static JWKS: LazyLock<JwkSet> = LazyLock::new(|| {
    let pem = std::fs::read(Config::signing_key_path()).unwrap_or_else(|error| {
        panic!(
            "SIGNING_KEY_PATH ({}) must be a readable file: {error}",
            Config::signing_key_path()
        )
    });

    jwks_from_pem(&pem).unwrap_or_else(|error| {
        panic!("SIGNING_KEY_PATH must contain an RSA private key in PEM format: {error}")
    })
});

/// Loads the signing key now, so a missing or invalid key stops the server at startup instead of
/// failing the first request.
pub fn init() {
    LazyLock::force(&JWKS);
}

pub fn router() -> Router {
    Router::new().route("/jwks.json", get(jwks_endpoint))
}

pub async fn jwks_endpoint() -> Response {
    (
        [(header::CACHE_CONTROL, HeaderValue::from_static(CACHE_CONTROL))],
        Json(&*JWKS),
    )
        .into_response()
}

/// Derives the public key set from an RSA private key in PEM format. Only the public parameters
/// (`n` and `e`) end up in the result. The `kid` is the RFC 7638 thumbprint of the public key, so
/// it changes exactly when the key does.
fn jwks_from_pem(pem: &[u8]) -> jsonwebtoken::errors::Result<JwkSet> {
    let key = EncodingKey::from_rsa_pem(pem)?;

    let mut jwk = Jwk::from_encoding_key(&key, SIGNING_ALGORITHM)?;
    jwk.common.public_key_use = Some(PublicKeyUse::Signature);
    jwk.common.key_id = Some(jwk.thumbprint(ThumbprintHash::SHA256)?);

    Ok(JwkSet { keys: vec![jwk] })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway 2048-bit key, generated for this test.
    const TEST_KEY: &str = include_str!("test_key.pem");

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

    #[tokio::test]
    async fn the_key_set_is_cacheable_json() {
        // Bypasses `JWKS`, which needs `SIGNING_KEY_PATH`: the headers do not depend on the key.
        let response = (
            [(header::CACHE_CONTROL, HeaderValue::from_static(CACHE_CONTROL))],
            Json(jwks_from_pem(TEST_KEY.as_bytes()).unwrap()),
        )
            .into_response();

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
