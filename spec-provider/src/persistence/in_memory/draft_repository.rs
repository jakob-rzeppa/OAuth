use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    application::contract::{ListQuery, RepositoryError, draft_repository::DraftRepository},
    domain::entity::spec::draft::Draft,
    persistence::in_memory::select_page,
};

/// Keeps drafts in memory. Everything is lost when the process stops.
#[derive(Default)]
pub struct InMemoryDraftRepository {
    drafts: RwLock<HashMap<Uuid, Draft>>,
}

#[async_trait]
impl DraftRepository for InMemoryDraftRepository {
    async fn register(&self, draft: &Draft) -> Result<(), RepositoryError> {
        let mut drafts = self.drafts.write().await;
        if drafts.contains_key(draft.id()) {
            return Err(RepositoryError::AlreadyExists);
        }
        drafts.insert(*draft.id(), draft.clone());
        Ok(())
    }

    async fn save(&self, draft: &Draft) -> Result<(), RepositoryError> {
        let mut drafts = self.drafts.write().await;
        let Some(stored) = drafts.get_mut(draft.id()) else {
            return Err(RepositoryError::NotFound);
        };
        *stored = draft.clone();
        Ok(())
    }

    async fn find_by_id(&self, draft_id: Uuid) -> Result<Option<Draft>, RepositoryError> {
        Ok(self.drafts.read().await.get(&draft_id).cloned())
    }

    async fn delete_by_id(&self, draft_id: Uuid) -> Result<(), RepositoryError> {
        match self.drafts.write().await.remove(&draft_id) {
            Some(_) => Ok(()),
            None => Err(RepositoryError::NotFound),
        }
    }

    async fn list(&self, query: &ListQuery) -> Result<Vec<Draft>, RepositoryError> {
        Ok(select_page(self.drafts.read().await.values(), query))
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::{
        application::contract::{PageCursor, SortDirection, SpecOrder},
        domain::entity::spec::draft::DraftVisibility,
    };

    fn make_draft(id: Uuid, title: &str) -> Draft {
        Draft::reconstitute(
            id,
            title.to_string(),
            "ABBR".to_string(),
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
    async fn register_then_find_returns_the_draft() {
        let repository = InMemoryDraftRepository::default();
        let id = Uuid::new_v4();

        repository.register(&make_draft(id, "Title")).await.unwrap();

        let found = repository.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(found.title(), "Title");
    }

    #[tokio::test]
    async fn register_fails_when_the_id_already_exists() {
        let repository = InMemoryDraftRepository::default();
        let id = Uuid::new_v4();
        repository.register(&make_draft(id, "Title")).await.unwrap();

        let result = repository.register(&make_draft(id, "Other")).await;

        assert_eq!(result, Err(RepositoryError::AlreadyExists));
    }

    #[tokio::test]
    async fn save_replaces_an_existing_draft() {
        let repository = InMemoryDraftRepository::default();
        let id = Uuid::new_v4();
        repository.register(&make_draft(id, "Title")).await.unwrap();

        repository.save(&make_draft(id, "Updated")).await.unwrap();

        let found = repository.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(found.title(), "Updated");
    }

    #[tokio::test]
    async fn save_fails_when_the_draft_does_not_exist() {
        let repository = InMemoryDraftRepository::default();

        let result = repository.save(&make_draft(Uuid::new_v4(), "Title")).await;

        assert_eq!(result, Err(RepositoryError::NotFound));
    }

    #[tokio::test]
    async fn delete_removes_the_draft() {
        let repository = InMemoryDraftRepository::default();
        let id = Uuid::new_v4();
        repository.register(&make_draft(id, "Title")).await.unwrap();

        repository.delete_by_id(id).await.unwrap();

        assert!(repository.find_by_id(id).await.unwrap().is_none());
        assert_eq!(
            repository.delete_by_id(id).await,
            Err(RepositoryError::NotFound)
        );
    }

    fn make_draft_at(id: Uuid, title: &str, abbreviation: &str, created_at_secs: i64) -> Draft {
        Draft::reconstitute(
            id,
            title.to_string(),
            abbreviation.to_string(),
            DraftVisibility::Public,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Utc.timestamp_opt(created_at_secs, 0).unwrap(),
        )
    }

    fn query() -> ListQuery {
        ListQuery {
            search: None,
            order: SpecOrder::default(),
            after: None,
            limit: 100,
        }
    }

    async fn repository_with(drafts: &[Draft]) -> InMemoryDraftRepository {
        let repository = InMemoryDraftRepository::default();
        for draft in drafts {
            repository.register(draft).await.unwrap();
        }
        repository
    }

    fn titles(drafts: &[Draft]) -> Vec<&str> {
        drafts.iter().map(|draft| draft.title()).collect()
    }

    #[tokio::test]
    async fn list_returns_the_newest_draft_first_by_default() {
        let repository = repository_with(&[
            make_draft_at(Uuid::new_v4(), "Old", "A", 10),
            make_draft_at(Uuid::new_v4(), "New", "B", 20),
        ])
        .await;

        let page = repository.list(&query()).await.unwrap();

        assert_eq!(titles(&page), ["New", "Old"]);
    }

    #[tokio::test]
    async fn list_returns_the_oldest_draft_first_when_ascending() {
        let repository = repository_with(&[
            make_draft_at(Uuid::new_v4(), "New", "B", 20),
            make_draft_at(Uuid::new_v4(), "Old", "A", 10),
        ])
        .await;
        let query = ListQuery {
            order: SpecOrder {
                direction: SortDirection::Ascending,
                ..SpecOrder::default()
            },
            ..query()
        };

        let page = repository.list(&query).await.unwrap();

        assert_eq!(titles(&page), ["Old", "New"]);
    }

    #[tokio::test]
    async fn list_continues_after_the_cursor_and_stops_at_the_limit() {
        let newest = Uuid::new_v4();
        let repository = repository_with(&[
            make_draft_at(newest, "Newest", "D", 40),
            make_draft_at(Uuid::new_v4(), "Newer", "C", 30),
            make_draft_at(Uuid::new_v4(), "Older", "B", 20),
            make_draft_at(Uuid::new_v4(), "Oldest", "A", 10),
        ])
        .await;
        let query = ListQuery {
            after: Some(PageCursor {
                created_at: Utc.timestamp_opt(40, 0).unwrap(),
                id: newest,
            }),
            limit: 2,
            ..query()
        };

        let page = repository.list(&query).await.unwrap();

        assert_eq!(titles(&page), ["Newer", "Older"]);
    }

    #[tokio::test]
    async fn list_only_returns_drafts_matching_the_search_in_title_or_abbreviation() {
        let repository = repository_with(&[
            make_draft_at(Uuid::new_v4(), "Open Banking", "OB", 10),
            make_draft_at(Uuid::new_v4(), "Payments", "BANK", 20),
            make_draft_at(Uuid::new_v4(), "Identity", "ID", 30),
        ])
        .await;
        let query = ListQuery {
            search: Some("bank".to_string()),
            ..query()
        };

        let page = repository.list(&query).await.unwrap();

        assert_eq!(titles(&page), ["Payments", "Open Banking"]);
    }
}
