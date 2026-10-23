use uuid::Uuid;

#[derive(Clone)]
pub struct AuthenticatedUser {
    client_id: Uuid,
    user_id: Uuid,

    scope: String,
}

impl AuthenticatedUser {
    pub fn new(client_id: Uuid, user_id: Uuid, scope: String) -> Self {
        Self {
            client_id,
            user_id,
            scope,
        }
    }

    pub fn client_id(&self) -> &Uuid {
        &self.client_id
    }

    pub fn user_id(&self) -> &Uuid {
        &self.user_id
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }
}
