use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct Client {
    id: Uuid,

    client_name: String,

    redirect_uris: Vec<String>,

    allowed_scopes: Vec<String>,
}

impl Client {
    pub fn new(
        id: Uuid,
        client_name: String,
        redirect_uris: Vec<String>,
        allowed_scopes: Vec<String>,
    ) -> Self {
        Self {
            id,
            client_name,
            redirect_uris,
            allowed_scopes,
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn client_name(&self) -> &str {
        &self.client_name
    }

    pub fn has_redirect_uri(&self, redirect_uri: &str) -> bool {
        self.redirect_uris.contains(&redirect_uri.to_string())
    }

    pub fn has_scope(&self, scopes: &str) -> bool {
        let requested_scopes: Vec<&str> = scopes.split_whitespace().collect();
        requested_scopes.iter().all(|requested| {
            self.allowed_scopes
                .iter()
                .any(|allowed| allowed == requested)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_redirect_uri() {
        let client = Client::new(
            Uuid::new_v4(),
            "Test Client".to_string(),
            vec!["https://example.com/callback".to_string()],
            vec!["read".to_string(), "write".to_string()],
        );

        assert!(client.has_redirect_uri("https://example.com/callback"));
        assert!(!client.has_redirect_uri("https://example.com/other"));
    }

    #[test]
    fn test_has_scopes() {
        let client = Client::new(
            Uuid::new_v4(),
            "Test Client".to_string(),
            vec!["https://example.com/callback".to_string()],
            vec!["read".to_string(), "write".to_string()],
        );

        assert!(client.has_scope("read"));
        assert!(client.has_scope("read write"));
        assert!(!client.has_scope("delete"));
        assert!(!client.has_scope("read delete"));
    }
}
