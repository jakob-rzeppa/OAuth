use std::sync::Arc;

use chrono::{DateTime, Utc};
use spec_api::{
    ListSpecsFilter, ListSpecsOrder, ListSpecsRequest, ListSpecsResponse, SortDirection, SpecKind,
    spec_list_item::Kind,
};

use crate::application::{
    contract::spec_gateway::{SpecGateway, SpecGatewayError},
    spec_status::SpecStatus,
    timestamp::TimestampService,
};

/// The number of specs loaded at once.
const PAGE_SIZE: u32 = 20;

/// What the overview lists.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ListQuery {
    /// Matched against the title and the abbreviation, empty for every spec.
    pub search: String,
    pub kind: KindFilter,
    pub order: Order,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum KindFilter {
    #[default]
    All,
    Published,
    Draft,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    #[default]
    Newest,
    Oldest,
}

impl ListQuery {
    fn to_request(&self, page_token: Option<String>) -> ListSpecsRequest {
        let kind = match self.kind {
            KindFilter::All => SpecKind::Both,
            KindFilter::Published => SpecKind::Published,
            KindFilter::Draft => SpecKind::Draft,
        };
        let direction = match self.order {
            Order::Newest => SortDirection::Desc,
            Order::Oldest => SortDirection::Asc,
        };
        ListSpecsRequest {
            page_size: PAGE_SIZE,
            page_token: page_token.unwrap_or_default(),
            filter: Some(ListSpecsFilter { kind: kind.into() }),
            search: self.search.trim().to_string(),
            order: Some(ListSpecsOrder {
                direction: direction.into(),
                ..Default::default()
            }),
        }
    }
}

/// One page of the overview.
#[derive(Debug)]
pub struct SpecListPage {
    pub entries: Vec<SpecListEntry>,
    /// `None` on the last page.
    pub next_page_token: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SpecListEntry {
    pub id: String,
    pub title: String,
    pub abbreviation: String,
    pub status: SpecStatus,
    /// `None` for a published spec, `Some(0)` for a draft without a version yet.
    pub current_version: Option<u32>,
    pub created_by: String,
    pub created_at: Option<DateTime<Utc>>,
}

impl SpecListPage {
    fn from_response(response: ListSpecsResponse, timestamps: &TimestampService) -> Self {
        SpecListPage {
            entries: response
                .items
                .into_iter()
                .filter_map(|item| item.kind)
                .map(|kind| SpecListEntry::from_kind(kind, timestamps))
                .collect(),
            next_page_token: Some(response.next_page_token).filter(|token| !token.is_empty()),
        }
    }
}

impl SpecListEntry {
    fn from_kind(kind: Kind, timestamps: &TimestampService) -> Self {
        match kind {
            Kind::Published(spec) => SpecListEntry {
                created_at: timestamps.to_time(spec.created_at.as_ref()),
                id: spec.id,
                title: spec.title,
                abbreviation: spec.abbreviation,
                status: SpecStatus::Published,
                current_version: None,
                created_by: spec.created_by,
            },
            Kind::Draft(draft) => SpecListEntry {
                created_at: timestamps.to_time(draft.created_at.as_ref()),
                status: SpecStatus::Draft {
                    visibility: draft.visibility().into(),
                },
                current_version: Some(draft.current_version),
                id: draft.id,
                title: draft.title,
                abbreviation: draft.abbreviation,
                created_by: draft.created_by,
            },
        }
    }
}

/// The use case of listing the published specs and the drafts.
pub struct SpecListService {
    spec_gateway: Arc<dyn SpecGateway>,
    timestamps: Arc<TimestampService>,
}

impl SpecListService {
    pub fn new(spec_gateway: Arc<dyn SpecGateway>, timestamps: Arc<TimestampService>) -> Self {
        SpecListService {
            spec_gateway,
            timestamps,
        }
    }

    /// Loads the page of the list starting at `page_token`, or the first page.
    pub async fn list_page(
        &self,
        query: &ListQuery,
        page_token: Option<String>,
    ) -> Result<SpecListPage, SpecGatewayError> {
        self.spec_gateway
            .list_specs(query.to_request(page_token))
            .await
            .map(|response| SpecListPage::from_response(response, &self.timestamps))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{contract::spec_gateway::MockSpecGateway, spec_status::Visibility};
    use spec_api::{DraftOverview, DraftVisibility, SpecListItem, SpecOverview};

    fn query(search: &str, kind: KindFilter, order: Order) -> ListQuery {
        ListQuery {
            search: search.into(),
            kind,
            order,
        }
    }

    #[test]
    fn maps_the_query_to_a_request() {
        let request =
            query(" bank ", KindFilter::Published, Order::Oldest).to_request(Some("next".into()));

        assert_eq!(request.page_size, PAGE_SIZE);
        assert_eq!(request.page_token, "next");
        assert_eq!(request.search, "bank");
        assert_eq!(request.filter.unwrap().kind(), SpecKind::Published);
        assert_eq!(request.order.unwrap().direction(), SortDirection::Asc);
    }

    #[test]
    fn requests_both_kinds_newest_first_by_default() {
        let request = ListQuery::default().to_request(None);

        assert_eq!(request.page_token, "");
        assert_eq!(request.filter.unwrap().kind(), SpecKind::Both);
        assert_eq!(request.order.unwrap().direction(), SortDirection::Desc);
    }

    #[test]
    fn maps_the_response_to_a_page() {
        let response = ListSpecsResponse {
            items: vec![
                SpecListItem {
                    kind: Some(Kind::Published(SpecOverview {
                        id: "1".into(),
                        title: "Spec".into(),
                        abbreviation: "S".into(),
                        created_by: "alice".into(),
                        created_at: Some(prost_types::Timestamp {
                            seconds: 1_705_000_000,
                            nanos: 0,
                        }),
                    })),
                },
                SpecListItem { kind: None },
                SpecListItem {
                    kind: Some(Kind::Draft(DraftOverview {
                        id: "2".into(),
                        visibility: DraftVisibility::Private.into(),
                        current_version: 3,
                        ..Default::default()
                    })),
                },
            ],
            next_page_token: "next".into(),
        };

        let page = SpecListPage::from_response(response, &TimestampService::new());

        assert_eq!(page.next_page_token.as_deref(), Some("next"));
        assert_eq!(page.entries.len(), 2);
        assert_eq!(
            page.entries[0].created_at,
            DateTime::from_timestamp(1_705_000_000, 0)
        );
        assert_eq!(page.entries[0].status, SpecStatus::Published);
        assert_eq!(page.entries[0].current_version, None);
        assert_eq!(
            page.entries[1].status,
            SpecStatus::Draft {
                visibility: Visibility::Private
            }
        );
        assert_eq!(page.entries[1].current_version, Some(3));
    }

    #[test]
    fn the_last_page_has_no_next_page_token() {
        let page =
            SpecListPage::from_response(ListSpecsResponse::default(), &TimestampService::new());

        assert!(page.entries.is_empty());
        assert_eq!(page.next_page_token, None);
    }

    #[tokio::test]
    async fn lists_a_page_through_the_gateway() {
        let mut spec_gateway = MockSpecGateway::new();
        spec_gateway
            .expect_list_specs()
            .withf(|request| request.page_token == "token" && request.search == "bank")
            .times(1)
            .returning(|_| {
                Ok(ListSpecsResponse {
                    items: vec![SpecListItem {
                        kind: Some(Kind::Published(SpecOverview::default())),
                    }],
                    next_page_token: "next".into(),
                })
            });
        let service =
            SpecListService::new(Arc::new(spec_gateway), Arc::new(TimestampService::new()));

        let page = service
            .list_page(
                &query("bank", KindFilter::All, Order::Newest),
                Some("token".into()),
            )
            .await
            .unwrap();

        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.next_page_token.as_deref(), Some("next"));
    }

    #[tokio::test]
    async fn passes_on_a_provider_error() {
        let mut spec_gateway = MockSpecGateway::new();
        spec_gateway
            .expect_list_specs()
            .returning(|_| Err(SpecGatewayError::Status(tonic::Status::unavailable("down"))));
        let service =
            SpecListService::new(Arc::new(spec_gateway), Arc::new(TimestampService::new()));

        let result = service.list_page(&ListQuery::default(), None).await;

        assert!(matches!(result, Err(SpecGatewayError::Status(_))));
    }
}
