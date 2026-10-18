use axum::{
    Json, Router,
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
    routing::get,
};
use jsonwebtoken::Algorithm;
use serde::Serialize;

use crate::{
    config::Config,
    domain::authorization_code::openid::{OPENID_SCOPE, PROFILE_SCOPE},
    jwks::SIGNING_ALGORITHM,
};

/// The metadata only changes with a deployment, so clients may cache it like the key set.
const CACHE_CONTROL: &str = "public, max-age=3600";

/// The OpenID Provider Metadata (OpenID Connect Discovery 1.0, section 3), extended with the
/// authorization server metadata of RFC 8414 and RFC 9126 for PAR and introspection.
#[derive(Debug, Serialize)]
pub struct ProviderMetadata {
    /// Must be identical to the `iss` of the ID tokens and authorization responses.
    issuer: &'static str,
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
    jwks_uri: String,
    pushed_authorization_request_endpoint: String,
    introspection_endpoint: String,

    /// Clients must push their authorization requests to the PAR endpoint first.
    require_pushed_authorization_requests: bool,
    scopes_supported: [&'static str; 2],
    response_types_supported: [&'static str; 1],
    response_modes_supported: [&'static str; 1],
    grant_types_supported: [&'static str; 1],
    code_challenge_methods_supported: [&'static str; 1],
    /// Only public clients exist so far.
    token_endpoint_auth_methods_supported: [&'static str; 1],
    introspection_endpoint_auth_methods_supported: [&'static str; 1],
    /// The `sub` is the user's id, the same for every client.
    subject_types_supported: [&'static str; 1],
    id_token_signing_alg_values_supported: [Algorithm; 1],
    claims_supported: [&'static str; 9],
    /// Authorization responses and error redirects carry `iss` (RFC 9207).
    authorization_response_iss_parameter_supported: bool,
}

impl ProviderMetadata {
    pub fn new(issuer: &'static str) -> Self {
        let endpoint = |path: &str| format!("{issuer}{path}");

        Self {
            issuer,
            authorization_endpoint: endpoint("/authorize"),
            token_endpoint: endpoint("/token"),
            userinfo_endpoint: endpoint("/userinfo"),
            jwks_uri: endpoint("/jwks.json"),
            pushed_authorization_request_endpoint: endpoint("/par"),
            introspection_endpoint: endpoint("/introspect"),
            require_pushed_authorization_requests: true,
            scopes_supported: [OPENID_SCOPE, PROFILE_SCOPE],
            response_types_supported: ["code"],
            response_modes_supported: ["query"],
            grant_types_supported: ["authorization_code"],
            code_challenge_methods_supported: ["S256"],
            token_endpoint_auth_methods_supported: ["none"],
            introspection_endpoint_auth_methods_supported: ["none"],
            subject_types_supported: ["public"],
            id_token_signing_alg_values_supported: [SIGNING_ALGORITHM],
            claims_supported: [
                "iss",
                "sub",
                "aud",
                "exp",
                "iat",
                "auth_time",
                "nonce",
                "name",
                "preferred_username",
            ],
            authorization_response_iss_parameter_supported: true,
        }
    }
}

pub fn router() -> Router {
    Router::new().route("/.well-known/openid-configuration", get(discovery_endpoint))
}

pub async fn discovery_endpoint() -> Response {
    (
        [(
            header::CACHE_CONTROL,
            HeaderValue::from_static(CACHE_CONTROL),
        )],
        Json(ProviderMetadata::new(Config::iss())),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use serde_json::Value;
    use tower::ServiceExt;

    const ISSUER: &str = "https://auth.example.com";

    async fn metadata() -> Value {
        Config::iss_fake().setup(|| ISSUER);

        let response = router()
            .oneshot(
                Request::get("/.well-known/openid-configuration")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn advertises_the_issuer_and_its_endpoints() {
        let metadata = metadata().await;

        assert_eq!(metadata["issuer"], ISSUER);
        assert_eq!(
            metadata["authorization_endpoint"],
            "https://auth.example.com/authorize"
        );
        assert_eq!(metadata["token_endpoint"], "https://auth.example.com/token");
        assert_eq!(
            metadata["userinfo_endpoint"],
            "https://auth.example.com/userinfo"
        );
        assert_eq!(metadata["jwks_uri"], "https://auth.example.com/jwks.json");
        assert_eq!(
            metadata["pushed_authorization_request_endpoint"],
            "https://auth.example.com/par"
        );
        assert_eq!(
            metadata["introspection_endpoint"],
            "https://auth.example.com/introspect"
        );
    }

    #[tokio::test]
    async fn advertises_the_code_flow_with_pkce_and_required_par() {
        let metadata = metadata().await;

        assert_eq!(metadata["require_pushed_authorization_requests"], true);
        assert_eq!(
            metadata["response_types_supported"],
            serde_json::json!(["code"])
        );
        assert_eq!(
            metadata["grant_types_supported"],
            serde_json::json!(["authorization_code"])
        );
        assert_eq!(
            metadata["code_challenge_methods_supported"],
            serde_json::json!(["S256"])
        );
        assert_eq!(
            metadata["authorization_response_iss_parameter_supported"],
            true
        );
    }

    #[tokio::test]
    async fn advertises_the_signing_algorithm_scopes_and_claims() {
        let metadata = metadata().await;

        assert_eq!(
            metadata["id_token_signing_alg_values_supported"],
            serde_json::json!(["RS256"])
        );
        assert_eq!(
            metadata["subject_types_supported"],
            serde_json::json!(["public"])
        );
        assert_eq!(
            metadata["scopes_supported"],
            serde_json::json!(["openid", "profile"])
        );
        let claims = metadata["claims_supported"].as_array().unwrap();
        for claim in [
            "sub",
            "iss",
            "auth_time",
            "nonce",
            "name",
            "preferred_username",
        ] {
            assert!(claims.contains(&Value::from(claim)), "{claim}");
        }
    }

    #[tokio::test]
    async fn the_metadata_is_cacheable_json() {
        Config::iss_fake().setup(|| ISSUER);

        let response = discovery_endpoint().await;

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
