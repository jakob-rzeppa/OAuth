use base64::Engine;
use chrono::Utc;
use sha2::Digest;

use crate::{
    api::token::{
        error_response::TokenErrorResponse, request::TokenRequest, response::TokenResponse,
    },
    persistence::access_tokens::register::register_access_token,
};

mod error_response;
mod generation;
mod request;
mod response;

mod grant {
    pub mod authorization_code;
}

#[axum::debug_handler]
pub async fn token_endpoint(request: TokenRequest) -> Result<TokenResponse, TokenErrorResponse> {
    let (token, access_token_entity) = match request.grant_type.as_str() {
        "authorization_code" => {
            grant::authorization_code::handle_authorization_code_grant(request).await?
        }
        _ => {
            tracing::warn!("token request with an unsupported grant_type");
            return Err(TokenErrorResponse::UnsupportedGrantType);
        }
    };

    register_access_token(&access_token_entity)
        .await
        .map_err(|_| TokenErrorResponse::DatabaseError)?;

    let expires_in = access_token_entity.exp().timestamp() - Utc::now().timestamp();
    tracing::info!(
        client_id = %access_token_entity.client_id(),
        scope = %access_token_entity.scope(),
        expires_in,
        "access token issued"
    );

    Ok(TokenResponse {
        access_token: token,
        token_type: "bearer".to_string(),
        expires_in,
        scope: access_token_entity.scope().to_string(),
    })
}

/// Verify the code_verifier against the code_challenge.
///
/// # code_challenge_method
///
/// "S256": S256 hashing and Base64URL encoding as per the PKCE specification.
fn verify_code_challenge(
    code_challenge_method: &str,
    code_verifier: &str,
    code_challenge: &str,
) -> bool {
    // Only support S256 method
    if code_challenge_method != "S256" {
        return false;
    }

    // 1. Hash the code_verifier using SHA256
    let hash = sha2::Sha256::digest(code_verifier.as_bytes());

    // 2. Base64URL encode the hash
    let encoded_hash = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash);

    // 3. Compare the result with the code_challenge
    encoded_hash == code_challenge
}

#[cfg(test)]
mod tests {
    use super::generation::generate_access_token_mock;
    use super::*;
    use crate::{
        domain::entity::{access_token::AccessToken, authorization_code::code::AuthorizationCode},
        logging::testing::LogCapture,
        persistence::{
            access_tokens::register::{RegisterAccessTokenError, register_access_token_fake},
            authorization_codes::take::take_authorization_code_fake,
        },
    };
    use uuid::Uuid;

    // The RFC 7636 appendix B example pair.
    const CODE_VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const CODE_CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

    fn token_request(client_id: Uuid) -> TokenRequest {
        TokenRequest {
            grant_type: "authorization_code".to_string(),
            client_id: client_id.to_string(),
            code: Some("the-code".to_string()),
            code_verifier: Some(CODE_VERIFIER.to_string()),
        }
    }

    /// An authorization code the grant accepts for `token_request`, and a generated token.
    fn setup_valid_grant(client_id: Uuid, user_id: Option<Uuid>) {
        take_authorization_code_fake().setup(move |_| {
            Ok(Some(AuthorizationCode::new(
                "the-code".to_string(),
                client_id,
                "read write".to_string(),
                Uuid::new_v4(),
                CODE_CHALLENGE.to_string(),
                "S256".to_string(),
            )))
        });
        generate_access_token_mock().setup(move |_, _, _| {
            (
                "secret-token-value".to_string(),
                AccessToken::new(
                    "token-hash".to_string(),
                    "bearer".to_string(),
                    client_id,
                    user_id,
                    chrono::Utc::now(),
                    chrono::Utc::now() + chrono::Duration::seconds(3600),
                    "read write".to_string(),
                ),
            )
        });
    }

    #[tokio::test]
    async fn logs_the_issued_token_without_its_value() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let client_id = Uuid::new_v4();
        let user_id = Some(Uuid::new_v4());
        setup_valid_grant(client_id, user_id);
        register_access_token_fake().setup(|_| Ok(()));

        let result = token_endpoint(token_request(client_id)).await;

        assert!(result.is_ok());
        let log = capture.contents();
        assert!(log.contains("INFO"), "{log}");
        assert!(log.contains("access token issued"), "{log}");
        assert!(log.contains(&client_id.to_string()), "{log}");
        assert!(log.contains("read write"), "{log}");
        assert!(!log.contains("secret-token-value"), "{log}");
        assert!(!log.contains("token-hash"), "{log}");
        assert!(!log.contains("the-code"), "{log}");
        assert!(!log.contains(CODE_VERIFIER), "{log}");
        assert!(!log.contains(CODE_CHALLENGE), "{log}");
    }

    #[tokio::test]
    async fn does_not_log_an_issued_token_when_registering_it_fails() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let client_id = Uuid::new_v4();
        let user_id = Some(Uuid::new_v4());
        setup_valid_grant(client_id, user_id);
        register_access_token_fake().setup(|_| Err(RegisterAccessTokenError::DatabaseError));

        let result = token_endpoint(token_request(client_id)).await;

        assert!(result.is_err());
        assert!(!capture.contents().contains("access token issued"));
    }

    #[tokio::test]
    async fn warns_about_an_unsupported_grant_type_without_logging_its_value() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let request = TokenRequest {
            grant_type: "password-grant-secret".to_string(),
            ..token_request(Uuid::new_v4())
        };

        let result = token_endpoint(request).await;

        assert!(result.is_err());
        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("unsupported grant_type"), "{log}");
        assert!(!log.contains("password-grant-secret"), "{log}");
    }

    #[tokio::test]
    async fn warns_when_the_code_verifier_does_not_match_the_code_challenge() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let client_id = Uuid::new_v4();
        let user_id = Some(Uuid::new_v4());
        setup_valid_grant(client_id, user_id);
        let request = TokenRequest {
            code_verifier: Some("a-wrong-verifier".to_string()),
            ..token_request(client_id)
        };

        let result = token_endpoint(request).await;

        assert!(result.is_err());
        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("code_verifier does not match"), "{log}");
        assert!(!log.contains("a-wrong-verifier"), "{log}");
        assert!(!log.contains("access token issued"), "{log}");
    }
}
