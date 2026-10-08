use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A logged-in user's session, stored under the hash of the session token held by the browser.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserSession {
    user_id: Uuid,
}

impl UserSession {
    pub fn new(user_id: Uuid) -> Self {
        Self { user_id }
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }
}
