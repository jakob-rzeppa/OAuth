use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    config::Config,
    util::token::{hash_token, random_token},
};

/// The only token type this server issues.
pub const BEARER: &str = "bearer";

pub struct AccessToken {
    token_hash: String,

    token_type: String,

    client_id: Uuid,

    /// The subject of the token, if any.
    /// This is the user ID of the user that the token was issued for.
    sub: Option<Uuid>,

    /// The time at which the token was issued, in UTC.
    iat: DateTime<Utc>,
    /// The time at which the token will expire, in UTC.
    exp: DateTime<Utc>,

    scope: String,
}

impl AccessToken {
    pub fn new(
        token_hash: String,
        token_type: String,
        client_id: Uuid,
        sub: Option<Uuid>,
        iat: DateTime<Utc>,
        exp: DateTime<Utc>,
        scope: String,
    ) -> Self {
        Self {
            token_hash,
            token_type,
            client_id,
            sub,
            iat,
            exp,
            scope,
        }
    }
}

impl AccessToken {
    pub fn token_hash(&self) -> &str {
        &self.token_hash
    }

    pub fn token_type(&self) -> &str {
        &self.token_type
    }

    pub fn client_id(&self) -> &Uuid {
        &self.client_id
    }

    pub fn sub(&self) -> Option<&Uuid> {
        self.sub.as_ref()
    }

    pub fn iat(&self) -> &DateTime<Utc> {
        &self.iat
    }

    pub fn exp(&self) -> &DateTime<Utc> {
        &self.exp
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }
}

/// Generates a new access token for `client_id`, on behalf of `user_id` if there is one.
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
    let exp = iat + chrono::Duration::seconds(Config::access_token_ttl().into());

    // Generate a unique token
    let token = random_token();
    let token_hash = hash_token(&token);

    let access_token = AccessToken::new(
        token_hash,
        BEARER.to_string(),
        client_id,
        user_id,
        iat,
        exp,
        scope.to_string(),
    );

    (token, access_token)
}
