use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use rand::Rng;
use sha2::Digest;
use uuid::Uuid;

use crate::{config::access_token_ttl, domain::entity::access_token::AccessToken};

/// Generates an access token based on the provided authorization code and client ID.
///
/// # Returns
///
/// 1. A `String` representing the generated access token.
/// 2. The AccessToken entity
#[fnmock::mockable]
pub fn generate_access_token(
    client_id: Uuid,
    user_id: Option<Uuid>,
    scope: &str,
) -> (String, AccessToken) {
    let iat = Utc::now();
    let exp = iat + chrono::Duration::seconds(access_token_ttl().into());

    // Generate a unique token
    let token = generate_token();
    let token_hash = hash_token(&token);

    let access_token = AccessToken::new(
        token_hash,
        "bearer".to_string(),
        client_id,
        user_id,
        Utc::now(),
        exp,
        scope.to_string(),
    );

    (token, access_token)
}

/// Generate a random 256-bit access token and encode it in URL-safe base64.
#[fnmock::fakeable]
fn generate_token() -> String {
    let mut bytes = [0u8; 32]; // 256 bits
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[fnmock::mockable]
fn hash_token(token: &str) -> String {
    let hash = sha2::Sha256::digest(token.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}
