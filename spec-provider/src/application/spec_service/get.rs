use uuid::Uuid;

use crate::{
    application::spec_service::SpecService,
    domain::projection::{draft::DraftProjection, spec::SpecProjection},
};

/// Specs and drafts share their IDs (publishing a draft keeps its ID),
/// so an ID resolves to either a published spec or a draft.
pub enum SpecOrDraftProjection {
    Spec(SpecProjection),
    Draft(DraftProjection),
}

#[derive(Debug, PartialEq)]
pub enum GetSpecError {
    SpecNotFound,
    DatabaseError,
}

impl SpecService {
    /// Returns the published spec with the given ID, or the draft if the spec isn't published.
    pub async fn get_spec_or_draft(&self, id: Uuid) -> Result<SpecOrDraftProjection, GetSpecError> {
        let spec = self
            .spec_repository
            .find_by_id(id)
            .await
            .map_err(|_| GetSpecError::DatabaseError)?;

        if let Some(spec) = spec {
            return Ok(SpecOrDraftProjection::Spec(SpecProjection::from(&spec)));
        }

        let draft = self
            .draft_repository
            .find_by_id(id)
            .await
            .map_err(|_| GetSpecError::DatabaseError)?;

        match draft {
            Some(draft) => Ok(SpecOrDraftProjection::Draft(DraftProjection::from(&draft))),
            None => Err(GetSpecError::SpecNotFound),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        application::contract::{
            draft_repository::DraftRepository, spec_repository::SpecRepository,
        },
        domain::entity::spec::{
            Spec,
            draft::{Draft, DraftVisibility},
        },
        persistence::in_memory::{
            draft_repository::InMemoryDraftRepository, spec_repository::InMemorySpecRepository,
        },
    };

    fn make_spec(id: Uuid) -> Spec {
        Spec::reconstitute(
            id,
            "Spec".to_string(),
            "SPEC".to_string(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
    }

    fn make_draft(id: Uuid) -> Draft {
        Draft::reconstitute(
            id,
            "Draft".to_string(),
            "DRAFT".to_string(),
            DraftVisibility::Public,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
    }

    #[tokio::test]
    async fn returns_the_spec_when_it_is_published() {
        let id = Uuid::new_v4();
        let specs = Arc::new(InMemorySpecRepository::default());
        specs.register(&make_spec(id)).await.unwrap();
        let service = SpecService::new(specs, Arc::new(InMemoryDraftRepository::default()));

        let result = service.get_spec_or_draft(id).await;

        let Ok(SpecOrDraftProjection::Spec(spec)) = result else {
            panic!("expected a spec");
        };
        assert_eq!(spec.id, id);
        assert_eq!(spec.title, "Spec");
    }

    #[tokio::test]
    async fn returns_the_draft_when_no_spec_exists() {
        let id = Uuid::new_v4();
        let drafts = Arc::new(InMemoryDraftRepository::default());
        drafts.register(&make_draft(id)).await.unwrap();
        let service = SpecService::new(Arc::new(InMemorySpecRepository::default()), drafts);

        let result = service.get_spec_or_draft(id).await;

        let Ok(SpecOrDraftProjection::Draft(draft)) = result else {
            panic!("expected a draft");
        };
        assert_eq!(draft.id, id);
        assert_eq!(draft.title, "Draft");
    }

    #[tokio::test]
    async fn fails_with_not_found_when_neither_exists() {
        let service = SpecService::new(
            Arc::new(InMemorySpecRepository::default()),
            Arc::new(InMemoryDraftRepository::default()),
        );

        let result = service.get_spec_or_draft(Uuid::new_v4()).await;

        assert!(matches!(result, Err(GetSpecError::SpecNotFound)));
    }
}
