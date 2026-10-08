use chrono::Utc;
use uuid::Uuid;

use crate::{
    config::access_token_ttl,
    domain::entity::access_token::{AccessToken, BEARER},
    util::token::{hash_token, random_token},
};

/// Generates an access token based on the provided authorization code and client ID.
///
/// # Returns
///
/// 1. A `String` representing the generated access token.
/// 2. The AccessToken entity
#[fnmock::mockable]
pub(super) fn generate_access_token(
    client_id: Uuid,
    user_id: Option<Uuid>,
    scope: &str,
) -> (String, AccessToken) {
    let iat = Utc::now();
    let exp = iat + chrono::Duration::seconds(access_token_ttl().into());

    // Generate a unique token
    let token = random_token();
    let token_hash = hash_token(&token);

    let access_token = AccessToken::new(
        token_hash,
        BEARER.to_string(),
        client_id,
        user_id,
        Utc::now(),
        exp,
        scope.to_string(),
    );

    (token, access_token)
}
