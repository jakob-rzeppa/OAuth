use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use e2e_tests::{Config, client};
use reqwest::Url;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const PAR_TTL_SECONDS: u64 = 180;
const CODE_TTL_SECONDS: u64 = 300;

/// Asserts that the JSON object has exactly the given fields, so a new or removed field is noticed.
fn assert_fields(body: &Value, expected: &[&str]) {
    let mut actual: Vec<&str> = body
        .as_object()
        .unwrap_or_else(|| panic!("expected a JSON object, got {body}"))
        .keys()
        .map(String::as_str)
        .collect();
    let mut expected = expected.to_vec();
    actual.sort_unstable();
    expected.sort_unstable();

    assert_eq!(actual, expected, "unexpected fields in {body}");
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// The full authorization code flow with PKCE, without the user interface: the authorization page
/// (`GET /authorize`) is skipped and the consent is submitted directly.
#[tokio::test]
async fn auth_flow() {
    let cfg = Config::load();
    let client = client();

    let state = "e2e-state";
    let code_verifier = "e2e-code-verifier-0123456789-0123456789-0123456789";
    let code_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));

    // ==== PAR ====

    let par_res = client
        .post(cfg.auth_url("/par"))
        .json(&json!({
            "client_id": cfg.client_id(),
            "redirect_uri": cfg.redirect_uri(),
            "response_type": "code",
            "scope": cfg.scope(),
            "state": state,
            "code_challenge": code_challenge,
            "code_challenge_method": "S256",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(par_res.status(), 201);
    assert_eq!(par_res.headers()["cache-control"], "no-store");
    let par: Value = par_res.json().await.unwrap();
    assert_fields(&par, &["request_uri", "expires_in"]);
    let request_uri = par["request_uri"].as_str().unwrap();
    assert!(
        request_uri.starts_with("urn:authorize:request_uri:"),
        "unexpected request_uri {request_uri}"
    );
    assert!(request_uri.len() > "urn:authorize:request_uri:".len());
    assert_eq!(par["expires_in"], PAR_TTL_SECONDS);

    // ==== Authorize ====

    let authorize_res = client
        .post(cfg.auth_url("/authorize"))
        .form(&[("request_uri", request_uri), ("client_id", cfg.client_id())])
        .send()
        .await
        .unwrap();

    assert_eq!(authorize_res.status(), 303);
    let location = authorize_res
        .headers()
        .get("location")
        .expect("the authorize response has no Location header")
        .to_str()
        .unwrap();
    let location = Url::parse(location).unwrap();
    let redirect_uri = Url::parse(cfg.redirect_uri()).unwrap();
    assert_eq!(location.scheme(), redirect_uri.scheme());
    assert_eq!(location.host_str(), redirect_uri.host_str());
    assert_eq!(location.port(), redirect_uri.port());
    assert_eq!(location.path(), redirect_uri.path());

    let mut params: HashMap<String, String> = location.query_pairs().into_owned().collect();
    let code = params.remove("code").expect("no code in the redirect");
    assert!(!code.is_empty());
    assert_eq!(params.remove("state").as_deref(), Some(state));
    assert_eq!(
        params.remove("expires_in").as_deref(),
        Some(CODE_TTL_SECONDS.to_string().as_str())
    );
    assert_eq!(params.remove("iss").as_deref(), Some(cfg.issuer()));
    assert!(params.is_empty(), "unexpected redirect params {params:?}");

    // ==== Token ====

    let token_res = client
        .post(cfg.auth_url("/token"))
        .json(&json!({
            "grant_type": "authorization_code",
            "client_id": cfg.client_id(),
            "code": code,
            "code_verifier": code_verifier,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(token_res.status(), 201);
    assert_eq!(token_res.headers()["cache-control"], "no-store");
    let token: Value = token_res.json().await.unwrap();
    assert_fields(
        &token,
        &["access_token", "token_type", "expires_in", "scope"],
    );
    let access_token = token["access_token"].as_str().unwrap();
    assert!(!access_token.is_empty());
    assert_eq!(token["token_type"], "bearer");
    assert_eq!(token["scope"], cfg.scope());
    let expires_in = token["expires_in"].as_i64().unwrap();

    // ==== Introspection ====

    let introspect_res = client
        .post(cfg.auth_url("/introspect"))
        .json(&json!({ "token": access_token }))
        .send()
        .await
        .unwrap();

    assert_eq!(introspect_res.status(), 200);
    assert_eq!(introspect_res.headers()["cache-control"], "no-store");
    let introspection: Value = introspect_res.json().await.unwrap();
    assert_fields(
        &introspection,
        &[
            "active",
            "scope",
            "client_id",
            "token_type",
            "iat",
            "exp",
            "iss",
        ],
    );
    assert_eq!(introspection["active"], true);
    assert_eq!(introspection["scope"], cfg.scope());
    assert_eq!(introspection["client_id"], cfg.client_id());
    assert_eq!(introspection["token_type"], "bearer");
    assert_eq!(introspection["iss"], cfg.issuer());

    let iat = introspection["iat"].as_i64().unwrap();
    let exp = introspection["exp"].as_i64().unwrap();
    assert!((now() - iat).abs() <= 10, "iat {iat} is not close to now");
    let ttl = exp - iat;
    assert!(ttl > 0, "the token expires before it was issued");
    // `expires_in` counts down from the issue time, so it can be a few seconds below the full ttl.
    assert!(
        (ttl - 5..=ttl).contains(&expires_in),
        "expires_in {expires_in} does not match the token lifetime {ttl}"
    );
}
