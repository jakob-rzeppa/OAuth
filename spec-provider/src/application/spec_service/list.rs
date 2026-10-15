use crate::{
    application::{
        contract::{ListQuery, PageCursor, SortDirection, SpecOrder},
        spec_service::SpecService,
    },
    domain::projection::{
        draft_overview::DraftOverviewProjection, spec_overview::SpecOverviewProjection,
    },
};

/// The page size used when the caller doesn't ask for one.
pub const DEFAULT_PAGE_SIZE: usize = 50;
/// The largest page size the service returns, however many items the caller asks for.
pub const MAX_PAGE_SIZE: usize = 100;

/// Which kinds of specs a listing contains.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpecKind {
    Published,
    Draft,
    Both,
}

/// Narrows down a listing. Add a field here for every new way to filter.
#[derive(Clone, Debug)]
pub struct ListSpecsFilter {
    pub kind: SpecKind,
}

pub struct ListSpecsParams {
    pub filter: ListSpecsFilter,
    /// Matched against title and abbreviation. Blank means no search.
    pub search: Option<String>,
    pub order: SpecOrder,
    /// The position to continue after, from the previous page's `next`.
    pub after: Option<PageCursor>,
    /// `None` selects `DEFAULT_PAGE_SIZE`. Values are clamped to `1..=MAX_PAGE_SIZE`.
    pub page_size: Option<usize>,
}

pub enum SpecOrDraftOverview {
    Spec(SpecOverviewProjection),
    Draft(DraftOverviewProjection),
}

pub struct SpecPage {
    pub items: Vec<SpecOrDraftOverview>,
    /// Where the next page starts, `None` on the last page.
    pub next: Option<PageCursor>,
}

#[derive(Debug, PartialEq)]
pub enum ListSpecsError {
    DatabaseError,
}

impl SpecService {
    /// Returns one page of published specs and/or drafts.
    pub async fn list_specs(&self, params: ListSpecsParams) -> Result<SpecPage, ListSpecsError> {
        let limit = params
            .page_size
            .unwrap_or(DEFAULT_PAGE_SIZE)
            .clamp(1, MAX_PAGE_SIZE);
        let search = params
            .search
            .map(|term| term.trim().to_string())
            .filter(|term| !term.is_empty());
        let query = ListQuery {
            search,
            order: params.order,
            after: params.after,
            // One more than the page, to learn whether there is a next page.
            limit: limit + 1,
        };

        // Each repository returns its own first `limit + 1` items in order, so the first
        // `limit + 1` items of the merged result are the first ones overall.
        let mut items: Vec<(PageCursor, SpecOrDraftOverview)> = Vec::new();

        if params.filter.kind != SpecKind::Draft {
            let specs = self
                .spec_repository
                .list(&query)
                .await
                .map_err(|_| ListSpecsError::DatabaseError)?;
            items.extend(specs.iter().map(|spec| {
                let cursor = PageCursor {
                    created_at: *spec.created_at(),
                    id: *spec.id(),
                };
                (
                    cursor,
                    SpecOrDraftOverview::Spec(SpecOverviewProjection::from(spec)),
                )
            }));
        }

        if params.filter.kind != SpecKind::Published {
            let drafts = self
                .draft_repository
                .list(&query)
                .await
                .map_err(|_| ListSpecsError::DatabaseError)?;
            items.extend(drafts.iter().map(|draft| {
                let cursor = PageCursor {
                    created_at: *draft.created_at(),
                    id: *draft.id(),
                };
                (
                    cursor,
                    SpecOrDraftOverview::Draft(DraftOverviewProjection::from(draft)),
                )
            }));
        }

        items.sort_by_key(|(cursor, _)| (cursor.created_at, cursor.id));
        if query.order.direction == SortDirection::Descending {
            items.reverse();
        }

        let has_more = items.len() > limit;
        items.truncate(limit);
        let next = if has_more {
            items.last().map(|(cursor, _)| *cursor)
        } else {
            None
        };

        Ok(SpecPage {
            items: items.into_iter().map(|(_, item)| item).collect(),
            next,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    use super::*;
    use crate::{
        application::contract::{
            SortDirection, draft_repository::DraftRepository, spec_repository::SpecRepository,
        },
        domain::entity::spec::{
            Spec,
            draft::{Draft, DraftVisibility},
        },
        persistence::in_memory::{
            draft_repository::InMemoryDraftRepository, spec_repository::InMemorySpecRepository,
        },
    };

    fn make_spec_at(title: &str, abbreviation: &str, created_at_secs: i64) -> Spec {
        Spec::reconstitute(
            Uuid::new_v4(),
            title.to_string(),
            abbreviation.to_string(),
            Default::default(),
            Default::default(),
            Default::default(),
            Utc.timestamp_opt(created_at_secs, 0).unwrap(),
        )
    }

    fn make_draft_at(title: &str, abbreviation: &str, created_at_secs: i64) -> Draft {
        Draft::reconstitute(
            Uuid::new_v4(),
            title.to_string(),
            abbreviation.to_string(),
            DraftVisibility::Private,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Utc.timestamp_opt(created_at_secs, 0).unwrap(),
        )
    }

    async fn service_with(specs: &[Spec], drafts: &[Draft]) -> SpecService {
        let spec_repository = Arc::new(InMemorySpecRepository::default());
        for spec in specs {
            spec_repository.register(spec).await.unwrap();
        }
        let draft_repository = Arc::new(InMemoryDraftRepository::default());
        for draft in drafts {
            draft_repository.register(draft).await.unwrap();
        }
        SpecService::new(spec_repository, draft_repository)
    }

    fn params() -> ListSpecsParams {
        ListSpecsParams {
            filter: ListSpecsFilter {
                kind: SpecKind::Both,
            },
            search: None,
            order: SpecOrder::default(),
            after: None,
            page_size: None,
        }
    }

    fn titles(page: &SpecPage) -> Vec<&str> {
        page.items
            .iter()
            .map(|item| match item {
                SpecOrDraftOverview::Spec(spec) => spec.title.as_str(),
                SpecOrDraftOverview::Draft(draft) => draft.title.as_str(),
            })
            .collect()
    }

    #[tokio::test]
    async fn lists_specs_and_drafts_together_newest_first() {
        let service = service_with(
            &[
                make_spec_at("spec-10", "S1", 10),
                make_spec_at("spec-30", "S3", 30),
            ],
            &[
                make_draft_at("draft-20", "D2", 20),
                make_draft_at("draft-40", "D4", 40),
            ],
        )
        .await;

        let page = service.list_specs(params()).await.unwrap();

        assert_eq!(
            titles(&page),
            ["draft-40", "spec-30", "draft-20", "spec-10"]
        );
        assert!(page.next.is_none());
    }

    #[tokio::test]
    async fn lists_the_oldest_first_when_ascending() {
        let service = service_with(
            &[
                make_spec_at("spec-10", "S1", 10),
                make_spec_at("spec-30", "S3", 30),
            ],
            &[make_draft_at("draft-20", "D2", 20)],
        )
        .await;
        let params = ListSpecsParams {
            order: SpecOrder {
                direction: SortDirection::Ascending,
                ..SpecOrder::default()
            },
            ..params()
        };

        let page = service.list_specs(params).await.unwrap();

        assert_eq!(titles(&page), ["spec-10", "draft-20", "spec-30"]);
    }

    #[tokio::test]
    async fn lists_only_published_specs_for_the_published_kind() {
        let service = service_with(
            &[make_spec_at("spec-10", "S1", 10)],
            &[make_draft_at("draft-20", "D2", 20)],
        )
        .await;
        let params = ListSpecsParams {
            filter: ListSpecsFilter {
                kind: SpecKind::Published,
            },
            ..params()
        };

        let page = service.list_specs(params).await.unwrap();

        assert_eq!(titles(&page), ["spec-10"]);
        assert!(matches!(page.items[0], SpecOrDraftOverview::Spec(_)));
    }

    #[tokio::test]
    async fn lists_only_drafts_for_the_draft_kind() {
        let service = service_with(
            &[make_spec_at("spec-10", "S1", 10)],
            &[make_draft_at("draft-20", "D2", 20)],
        )
        .await;
        let params = ListSpecsParams {
            filter: ListSpecsFilter {
                kind: SpecKind::Draft,
            },
            ..params()
        };

        let page = service.list_specs(params).await.unwrap();

        assert_eq!(titles(&page), ["draft-20"]);
        assert!(matches!(page.items[0], SpecOrDraftOverview::Draft(_)));
    }

    #[tokio::test]
    async fn lists_drafts_of_every_visibility() {
        let drafts: Vec<Draft> = [
            DraftVisibility::Public,
            DraftVisibility::Private,
            DraftVisibility::Hidden,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, visibility)| {
            Draft::reconstitute(
                Uuid::new_v4(),
                format!("draft-{i}"),
                "D".to_string(),
                visibility,
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Utc.timestamp_opt(i as i64, 0).unwrap(),
            )
        })
        .collect();
        let service = service_with(&[], &drafts).await;

        let page = service.list_specs(params()).await.unwrap();

        assert_eq!(page.items.len(), 3);
    }

    #[tokio::test]
    async fn searches_titles_and_abbreviations_of_specs_and_drafts() {
        let service = service_with(
            &[
                make_spec_at("Open Banking", "OB", 10),
                make_spec_at("Identity", "ID", 20),
            ],
            &[
                make_draft_at("Payments", "BANK", 30),
                make_draft_at("Tokens", "TK", 40),
            ],
        )
        .await;
        let params = ListSpecsParams {
            search: Some("bank".to_string()),
            ..params()
        };

        let page = service.list_specs(params).await.unwrap();

        assert_eq!(titles(&page), ["Payments", "Open Banking"]);
    }

    #[tokio::test]
    async fn ignores_a_blank_search() {
        let service = service_with(
            &[make_spec_at("spec-10", "S1", 10)],
            &[make_draft_at("draft-20", "D2", 20)],
        )
        .await;
        let params = ListSpecsParams {
            search: Some("  \t".to_string()),
            ..params()
        };

        let page = service.list_specs(params).await.unwrap();

        assert_eq!(titles(&page), ["draft-20", "spec-10"]);
    }

    #[tokio::test]
    async fn pages_through_specs_and_drafts_without_gaps_or_repeats() {
        let service = service_with(
            &[
                make_spec_at("spec-10", "S", 10),
                make_spec_at("spec-30", "S", 30),
                make_spec_at("spec-50", "S", 50),
            ],
            &[
                make_draft_at("draft-20", "D", 20),
                make_draft_at("draft-40", "D", 40),
                make_draft_at("draft-60", "D", 60),
            ],
        )
        .await;

        let first = service
            .list_specs(ListSpecsParams {
                page_size: Some(2),
                ..params()
            })
            .await
            .unwrap();
        let second = service
            .list_specs(ListSpecsParams {
                page_size: Some(2),
                after: first.next,
                ..params()
            })
            .await
            .unwrap();
        let third = service
            .list_specs(ListSpecsParams {
                page_size: Some(2),
                after: second.next,
                ..params()
            })
            .await
            .unwrap();

        assert_eq!(titles(&first), ["draft-60", "spec-50"]);
        assert_eq!(titles(&second), ["draft-40", "spec-30"]);
        assert_eq!(titles(&third), ["draft-20", "spec-10"]);
        assert!(first.next.is_some());
        assert!(second.next.is_some());
        assert!(third.next.is_none());
    }

    #[tokio::test]
    async fn next_points_at_the_last_item_of_the_page() {
        let spec = make_spec_at("spec-10", "S", 10);
        let draft = make_draft_at("draft-20", "D", 20);
        let extra = make_spec_at("spec-5", "S", 5);
        let service = service_with(&[spec.clone(), extra], &[draft]).await;

        let page = service
            .list_specs(ListSpecsParams {
                page_size: Some(2),
                ..params()
            })
            .await
            .unwrap();

        assert_eq!(
            page.next,
            Some(PageCursor {
                created_at: *spec.created_at(),
                id: *spec.id(),
            })
        );
    }

    #[tokio::test]
    async fn has_no_next_page_when_the_items_exactly_fill_the_page() {
        let service = service_with(
            &[make_spec_at("spec-10", "S", 10)],
            &[make_draft_at("draft-20", "D", 20)],
        )
        .await;

        let page = service
            .list_specs(ListSpecsParams {
                page_size: Some(2),
                ..params()
            })
            .await
            .unwrap();

        assert_eq!(page.items.len(), 2);
        assert!(page.next.is_none());
    }

    #[tokio::test]
    async fn uses_the_default_page_size_when_none_is_given() {
        let specs: Vec<Spec> = (0..DEFAULT_PAGE_SIZE as i64 + 1)
            .map(|i| make_spec_at("spec", "S", i))
            .collect();
        let service = service_with(&specs, &[]).await;

        let page = service.list_specs(params()).await.unwrap();

        assert_eq!(page.items.len(), DEFAULT_PAGE_SIZE);
        assert!(page.next.is_some());
    }

    #[tokio::test]
    async fn caps_the_page_size_at_the_maximum() {
        let specs: Vec<Spec> = (0..MAX_PAGE_SIZE as i64 + 1)
            .map(|i| make_spec_at("spec", "S", i))
            .collect();
        let service = service_with(&specs, &[]).await;

        let page = service
            .list_specs(ListSpecsParams {
                page_size: Some(MAX_PAGE_SIZE * 10),
                ..params()
            })
            .await
            .unwrap();

        assert_eq!(page.items.len(), MAX_PAGE_SIZE);
        assert!(page.next.is_some());
    }

    #[tokio::test]
    async fn returns_at_least_one_item_when_the_page_size_is_zero() {
        let service = service_with(
            &[
                make_spec_at("spec-10", "S", 10),
                make_spec_at("spec-20", "S", 20),
            ],
            &[],
        )
        .await;

        let page = service
            .list_specs(ListSpecsParams {
                page_size: Some(0),
                ..params()
            })
            .await
            .unwrap();

        assert_eq!(titles(&page), ["spec-20"]);
    }
}
