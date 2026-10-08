use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::domain::entity::access_token::AccessToken;

pub mod find_by_token_hash;
pub mod register;
pub mod remove;

#[derive(FromRow)]
struct AccessTokenRow {
    token_hash: String,
    token_type: String,
    client_id: Uuid,
    sub: Option<Uuid>,
    iat: DateTime<Utc>,
    exp: DateTime<Utc>,
    scope: String,
}

impl AccessTokenRow {
    fn into_access_token(self) -> AccessToken {
        AccessToken::new(
            self.token_hash,
            self.token_type,
            self.client_id,
            self.sub,
            self.iat,
            self.exp,
            self.scope,
        )
    }
}
