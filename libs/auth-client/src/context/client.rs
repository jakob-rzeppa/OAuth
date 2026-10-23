use uuid::Uuid;

#[derive(Clone)]
pub struct AuthenticatedClient {
    client_id: Uuid,

    scope: String,
}

impl AuthenticatedClient {
    pub fn new(client_id: Uuid, scope: String) -> Self {
        Self { client_id, scope }
    }

    pub fn client_id(&self) -> &Uuid {
        &self.client_id
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }
}
