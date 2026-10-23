use crate::context::{client::AuthenticatedClient, user::AuthenticatedUser};

pub mod client;
pub mod user;

#[derive(Clone)]
pub enum AuthContext {
    Client(AuthenticatedClient),
    User(AuthenticatedUser),
}

impl AuthContext {
    /// Returns true if the AuthContext is a client acting by itself.
    pub fn is_client(&self) -> bool {
        matches!(self, AuthContext::Client(_))
    }

    /// Returns true if the AuthContext is a client acting on behalf of a user.
    pub fn is_user(&self) -> bool {
        matches!(self, AuthContext::User(_))
    }

    pub fn client_id(&self) -> &uuid::Uuid {
        match self {
            AuthContext::Client(client) => client.client_id(),
            AuthContext::User(user) => user.client_id(),
        }
    }

    /// If the client is acting on behalf of a user return the User Id, else None.
    pub fn user_id(&self) -> Option<&uuid::Uuid> {
        match self {
            AuthContext::Client(_) => None,
            AuthContext::User(user) => Some(&user.user_id()),
        }
    }

    fn scope(&self) -> &str {
        match self {
            AuthContext::Client(client) => client.scope(),
            AuthContext::User(user) => user.scope(),
        }
    }

    /// Returns true if the AuthContext has the required scope.
    pub fn has_scope(&self, required_scope: &str) -> bool {
        self.scope()
            .split_whitespace()
            .any(|scope| scope == required_scope)
    }

    /// Returns true if the AuthContext has all of the required scopes.
    pub fn has_scopes(&self, required_scopes: &[&str]) -> bool {
        required_scopes.iter().all(|scope| self.has_scope(scope))
    }
}
