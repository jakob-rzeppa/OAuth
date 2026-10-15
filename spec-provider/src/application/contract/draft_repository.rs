use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::contract::{ListQuery, RepositoryError},
    domain::entity::spec::draft::Draft,
};

#[async_trait]
pub trait DraftRepository: Send + Sync {
    /// Stores a new draft.
    async fn register(&self, draft: &Draft) -> Result<(), RepositoryError>;

    /// Updates an existing draft.
    async fn save(&self, draft: &Draft) -> Result<(), RepositoryError>;

    async fn find_by_id(&self, draft_id: Uuid) -> Result<Option<Draft>, RepositoryError>;

    async fn delete_by_id(&self, draft_id: Uuid) -> Result<(), RepositoryError>;

    /// Returns one page of drafts matching the query, in the query's order.
    async fn list(&self, query: &ListQuery) -> Result<Vec<Draft>, RepositoryError>;
}
