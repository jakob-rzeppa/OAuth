use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::contract::{ListQuery, RepositoryError},
    domain::entity::spec::Spec,
};

#[async_trait]
pub trait SpecRepository: Send + Sync {
    /// Stores a new spec.
    async fn register(&self, spec: &Spec) -> Result<(), RepositoryError>;

    /// Updates an existing spec.
    async fn save(&self, spec: &Spec) -> Result<(), RepositoryError>;

    async fn find_by_id(&self, spec_id: Uuid) -> Result<Option<Spec>, RepositoryError>;

    async fn delete_by_id(&self, spec_id: Uuid) -> Result<(), RepositoryError>;

    /// Returns one page of specs matching the query, in the query's order.
    async fn list(&self, query: &ListQuery) -> Result<Vec<Spec>, RepositoryError>;
}
