use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum GetContentError {
    #[error("failed to fetch content: {0}")]
    Request(String),
    #[error("content storage {1} answered with {0}")]
    Status(u16, String),
}

/// The content storage, which holds the full texts of the patches and proposals.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ContentGateway: Send + Sync {
    /// Fetches the full text behind the `content_uri` of a patch or proposal.
    async fn get(&self, uri: &str) -> Result<String, GetContentError>;
}
