use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A logged-in user's session, stored under the hash of the session token held by the browser.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserSession {
    user_id: Uuid,
    /// When the user logged in. Not renewed when the session is accessed.
    authenticated_at: DateTime<Utc>,
}

impl UserSession {
    pub fn new(user_id: Uuid, authenticated_at: DateTime<Utc>) -> Self {
        Self {
            user_id,
            authenticated_at,
        }
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    pub fn authenticated_at(&self) -> DateTime<Utc> {
        self.authenticated_at
    }
}
