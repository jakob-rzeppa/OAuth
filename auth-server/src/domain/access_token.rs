use chrono::{DateTime, Utc};
use uuid::Uuid;

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
