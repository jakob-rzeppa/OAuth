use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    application::contract::{ListQuery, RepositoryError, spec_repository::SpecRepository},
    domain::entity::spec::Spec,
    persistence::in_memory::select_page,
};

/// Keeps specs in memory. Everything is lost when the process stops.
#[derive(Default)]
pub struct InMemorySpecRepository {
    specs: RwLock<HashMap<Uuid, Spec>>,
}

#[async_trait]
impl SpecRepository for InMemorySpecRepository {
    async fn register(&self, spec: &Spec) -> Result<(), RepositoryError> {
        let mut specs = self.specs.write().await;
        if specs.contains_key(spec.id()) {
            return Err(RepositoryError::AlreadyExists);
        }
        specs.insert(*spec.id(), spec.clone());
        Ok(())
    }

    async fn save(&self, spec: &Spec) -> Result<(), RepositoryError> {
        let mut specs = self.specs.write().await;
        let Some(stored) = specs.get_mut(spec.id()) else {
            return Err(RepositoryError::NotFound);
        };
        *stored = spec.clone();
        Ok(())
    }

    async fn find_by_id(&self, spec_id: Uuid) -> Result<Option<Spec>, RepositoryError> {
        Ok(self.specs.read().await.get(&spec_id).cloned())
    }

    async fn delete_by_id(&self, spec_id: Uuid) -> Result<(), RepositoryError> {
        match self.specs.write().await.remove(&spec_id) {
            Some(_) => Ok(()),
            None => Err(RepositoryError::NotFound),
        }
    }

    async fn list(&self, query: &ListQuery) -> Result<Vec<Spec>, RepositoryError> {
        Ok(select_page(self.specs.read().await.values(), query))
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::application::contract::{PageCursor, SortDirection, SpecOrder};

    fn make_spec(id: Uuid, title: &str) -> Spec {
        Spec::reconstitute(
            id,
            title.to_string(),
            "ABBR".to_string(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
    }

    #[tokio::test]
    async fn register_then_find_returns_the_spec() {
        let repository = InMemorySpecRepository::default();
        let id = Uuid::new_v4();

        repository.register(&make_spec(id, "Title")).await.unwrap();

        let found = repository.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(found.title(), "Title");
    }

    #[tokio::test]
    async fn register_fails_when_the_id_already_exists() {
        let repository = InMemorySpecRepository::default();
        let id = Uuid::new_v4();
        repository.register(&make_spec(id, "Title")).await.unwrap();

        let result = repository.register(&make_spec(id, "Other")).await;

        assert_eq!(result, Err(RepositoryError::AlreadyExists));
    }

    #[tokio::test]
    async fn save_replaces_an_existing_spec() {
        let repository = InMemorySpecRepository::default();
        let id = Uuid::new_v4();
        repository.register(&make_spec(id, "Title")).await.unwrap();

        repository.save(&make_spec(id, "Updated")).await.unwrap();

        let found = repository.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(found.title(), "Updated");
    }

    #[tokio::test]
    async fn save_fails_when_the_spec_does_not_exist() {
        let repository = InMemorySpecRepository::default();

        let result = repository.save(&make_spec(Uuid::new_v4(), "Title")).await;

        assert_eq!(result, Err(RepositoryError::NotFound));
    }

    #[tokio::test]
    async fn delete_removes_the_spec() {
        let repository = InMemorySpecRepository::default();
        let id = Uuid::new_v4();
        repository.register(&make_spec(id, "Title")).await.unwrap();

        repository.delete_by_id(id).await.unwrap();

        assert!(repository.find_by_id(id).await.unwrap().is_none());
        assert_eq!(
            repository.delete_by_id(id).await,
            Err(RepositoryError::NotFound)
        );
    }

    fn make_spec_at(id: Uuid, title: &str, abbreviation: &str, created_at_secs: i64) -> Spec {
        Spec::reconstitute(
            id,
            title.to_string(),
            abbreviation.to_string(),
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

    async fn repository_with(specs: &[Spec]) -> InMemorySpecRepository {
        let repository = InMemorySpecRepository::default();
        for spec in specs {
            repository.register(spec).await.unwrap();
        }
        repository
    }

    fn titles(specs: &[Spec]) -> Vec<&str> {
        specs.iter().map(|spec| spec.title()).collect()
    }

    #[tokio::test]
    async fn list_returns_the_newest_spec_first_by_default() {
        let repository = repository_with(&[
            make_spec_at(Uuid::new_v4(), "Middle", "B", 20),
            make_spec_at(Uuid::new_v4(), "Newest", "C", 30),
            make_spec_at(Uuid::new_v4(), "Oldest", "A", 10),
        ])
        .await;

        let page = repository.list(&query()).await.unwrap();

        assert_eq!(titles(&page), ["Newest", "Middle", "Oldest"]);
    }

    #[tokio::test]
    async fn list_returns_the_oldest_spec_first_when_ascending() {
        let repository = repository_with(&[
            make_spec_at(Uuid::new_v4(), "Middle", "B", 20),
            make_spec_at(Uuid::new_v4(), "Newest", "C", 30),
            make_spec_at(Uuid::new_v4(), "Oldest", "A", 10),
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

        assert_eq!(titles(&page), ["Oldest", "Middle", "Newest"]);
    }

    #[tokio::test]
    async fn list_breaks_ties_by_id_in_the_direction_of_the_order() {
        let low = Uuid::from_u128(1);
        let high = Uuid::from_u128(2);
        let repository = repository_with(&[
            make_spec_at(high, "High", "H", 10),
            make_spec_at(low, "Low", "L", 10),
        ])
        .await;
        let ascending = ListQuery {
            order: SpecOrder {
                direction: SortDirection::Ascending,
                ..SpecOrder::default()
            },
            ..query()
        };

        let descending_page = repository.list(&query()).await.unwrap();
        let ascending_page = repository.list(&ascending).await.unwrap();

        assert_eq!(titles(&descending_page), ["High", "Low"]);
        assert_eq!(titles(&ascending_page), ["Low", "High"]);
    }

    #[tokio::test]
    async fn list_returns_at_most_limit_specs() {
        let repository = repository_with(&[
            make_spec_at(Uuid::new_v4(), "One", "A", 10),
            make_spec_at(Uuid::new_v4(), "Two", "B", 20),
            make_spec_at(Uuid::new_v4(), "Three", "C", 30),
        ])
        .await;
        let query = ListQuery {
            limit: 2,
            ..query()
        };

        let page = repository.list(&query).await.unwrap();

        assert_eq!(titles(&page), ["Three", "Two"]);
    }

    #[tokio::test]
    async fn list_continues_after_the_cursor_in_descending_order() {
        let middle = Uuid::new_v4();
        let repository = repository_with(&[
            make_spec_at(Uuid::new_v4(), "Newest", "C", 30),
            make_spec_at(middle, "Middle", "B", 20),
            make_spec_at(Uuid::new_v4(), "Oldest", "A", 10),
        ])
        .await;
        let query = ListQuery {
            after: Some(PageCursor {
                created_at: Utc.timestamp_opt(20, 0).unwrap(),
                id: middle,
            }),
            ..query()
        };

        let page = repository.list(&query).await.unwrap();

        assert_eq!(titles(&page), ["Oldest"]);
    }

    #[tokio::test]
    async fn list_continues_after_the_cursor_in_ascending_order() {
        let middle = Uuid::new_v4();
        let repository = repository_with(&[
            make_spec_at(Uuid::new_v4(), "Newest", "C", 30),
            make_spec_at(middle, "Middle", "B", 20),
            make_spec_at(Uuid::new_v4(), "Oldest", "A", 10),
        ])
        .await;
        let query = ListQuery {
            order: SpecOrder {
                direction: SortDirection::Ascending,
                ..SpecOrder::default()
            },
            after: Some(PageCursor {
                created_at: Utc.timestamp_opt(20, 0).unwrap(),
                id: middle,
            }),
            ..query()
        };

        let page = repository.list(&query).await.unwrap();

        assert_eq!(titles(&page), ["Newest"]);
    }

    #[tokio::test]
    async fn list_continues_between_specs_with_the_same_created_at() {
        let low = Uuid::from_u128(1);
        let high = Uuid::from_u128(2);
        let repository = repository_with(&[
            make_spec_at(high, "High", "H", 10),
            make_spec_at(low, "Low", "L", 10),
        ])
        .await;
        let query = ListQuery {
            after: Some(PageCursor {
                created_at: Utc.timestamp_opt(10, 0).unwrap(),
                id: high,
            }),
            ..query()
        };

        let page = repository.list(&query).await.unwrap();

        assert_eq!(titles(&page), ["Low"]);
    }

    #[tokio::test]
    async fn list_only_returns_specs_matching_the_search_in_title_or_abbreviation() {
        let repository = repository_with(&[
            make_spec_at(Uuid::new_v4(), "Open Banking", "OB", 10),
            make_spec_at(Uuid::new_v4(), "Payments", "BANK", 20),
            make_spec_at(Uuid::new_v4(), "Identity", "ID", 30),
        ])
        .await;
        let query = ListQuery {
            search: Some("bank".to_string()),
            ..query()
        };

        let page = repository.list(&query).await.unwrap();

        assert_eq!(titles(&page), ["Payments", "Open Banking"]);
    }

    #[tokio::test]
    async fn list_applies_the_search_before_the_limit() {
        let repository = repository_with(&[
            make_spec_at(Uuid::new_v4(), "Match old", "A", 10),
            make_spec_at(Uuid::new_v4(), "Other", "B", 20),
            make_spec_at(Uuid::new_v4(), "Match new", "C", 30),
        ])
        .await;
        let query = ListQuery {
            search: Some("match".to_string()),
            limit: 2,
            ..query()
        };

        let page = repository.list(&query).await.unwrap();

        assert_eq!(titles(&page), ["Match new", "Match old"]);
    }
}
