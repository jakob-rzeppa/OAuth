use std::sync::Arc;

use serde::Deserialize;
use spec_api::{
    ListSpecsFilter, ListSpecsOrder, ListSpecsRequest, ListSpecsResponse, SortDirection, SpecKind,
    spec_list_item::Kind,
};

use crate::application::{
    contract::spec_gateway::{SpecGateway, SpecGatewayError},
    spec_status::SpecStatus,
    timestamp::format_date,
};

/// The number of specs loaded at once.
const PAGE_SIZE: u32 = 20;

/// The search, filters and order of the overview, as kept in the url.
///
/// Unknown values fall back to the default, so an edited url never fails.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct ListQuery {
    #[serde(rename = "q")]
    pub search: String,
    pub kind: KindFilter,
    pub order: Order,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(from = "String")]
pub enum KindFilter {
    #[default]
    All,
    Published,
    Draft,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(from = "String")]
pub enum Order {
    #[default]
    Newest,
    Oldest,
}

impl From<String> for KindFilter {
    fn from(value: String) -> Self {
        match value.as_str() {
            "published" => KindFilter::Published,
            "draft" => KindFilter::Draft,
            _ => KindFilter::All,
        }
    }
}

impl KindFilter {
    pub fn as_str(&self) -> &'static str {
        match self {
            KindFilter::All => "all",
            KindFilter::Published => "published",
            KindFilter::Draft => "draft",
        }
    }
}

impl From<String> for Order {
    fn from(value: String) -> Self {
        match value.as_str() {
            "oldest" => Order::Oldest,
            _ => Order::Newest,
        }
    }
}

impl Order {
    pub fn as_str(&self) -> &'static str {
        match self {
            Order::Newest => "newest",
            Order::Oldest => "oldest",
        }
    }
}

impl ListQuery {
    /// The url query of this list, without the defaults so urls stay short.
    pub fn query_string(&self) -> String {
        self.query_string_with_page_token(None)
    }

    /// The url query of the page of this list starting at `page_token`.
    pub fn query_string_with_page_token(&self, page_token: Option<&str>) -> String {
        let search = self.search.trim();
        let mut pairs = Vec::new();
        if !search.is_empty() {
            pairs.push(("q", search));
        }
        if self.kind != KindFilter::default() {
            pairs.push(("kind", self.kind.as_str()));
        }
        if self.order != Order::default() {
            pairs.push(("order", self.order.as_str()));
        }
        if let Some(page_token) = page_token {
            pairs.push(("page_token", page_token));
        }
        serde_urlencoded::to_string(pairs).expect("string pairs always encode")
    }

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
    pub created_at: String,
}

impl SpecListEntry {
    /// Like `Version 3`, or `No version yet`. `None` for a published spec.
    pub fn current_version_label(&self) -> Option<String> {
        self.current_version.map(|version| match version {
            0 => "No version yet".to_string(),
            version => format!("Version {version}"),
        })
    }
}

impl From<ListSpecsResponse> for SpecListPage {
    fn from(response: ListSpecsResponse) -> Self {
        SpecListPage {
            entries: response
                .items
                .into_iter()
                .filter_map(|item| item.kind)
                .map(SpecListEntry::from)
                .collect(),
            next_page_token: Some(response.next_page_token).filter(|token| !token.is_empty()),
        }
    }
}

impl From<Kind> for SpecListEntry {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Published(spec) => SpecListEntry {
                created_at: format_date(spec.created_at.as_ref()),
                id: spec.id,
                title: spec.title,
                abbreviation: spec.abbreviation,
                status: SpecStatus::Published,
                current_version: None,
                created_by: spec.created_by,
            },
            Kind::Draft(draft) => SpecListEntry {
                created_at: format_date(draft.created_at.as_ref()),
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
}

impl SpecListService {
    pub fn new(spec_gateway: Arc<dyn SpecGateway>) -> Self {
        SpecListService { spec_gateway }
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
            .map(SpecListPage::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{contract::spec_gateway::MockSpecGateway, spec_status::Visibility};
    use spec_api::{DraftOverview, DraftVisibility, SpecListItem, SpecOverview};

    fn parse(query: &str) -> ListQuery {
        serde_urlencoded::from_str(query).unwrap()
    }

    #[test]
    fn parses_the_url_query() {
        assert_eq!(
            parse("q=bank&kind=draft&order=oldest"),
            ListQuery {
                search: "bank".into(),
                kind: KindFilter::Draft,
                order: Order::Oldest,
            }
        );
    }

    #[test]
    fn falls_back_to_the_defaults_for_missing_and_unknown_values() {
        assert_eq!(parse(""), ListQuery::default());
        assert_eq!(parse("kind=everything&order=random"), ListQuery::default());
    }

    #[test]
    fn leaves_the_defaults_out_of_the_query_string() {
        assert_eq!(parse("q=+&kind=all&order=newest").query_string(), "");
        assert_eq!(
            parse("q=a b&kind=published&order=oldest").query_string_with_page_token(Some("7")),
            "q=a+b&kind=published&order=oldest&page_token=7"
        );
    }

    #[test]
    fn maps_the_query_to_a_request() {
        let request = parse("q= bank &kind=published&order=oldest").to_request(Some("next".into()));

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

        let page = SpecListPage::from(response);

        assert_eq!(page.next_page_token.as_deref(), Some("next"));
        assert_eq!(page.entries.len(), 2);
        assert_eq!(page.entries[0].created_at, "2024-01-11");
        assert_eq!(page.entries[0].status, SpecStatus::Published);
        assert_eq!(page.entries[0].current_version_label(), None);
        assert_eq!(
            page.entries[1].status,
            SpecStatus::Draft {
                visibility: Visibility::Private
            }
        );
        assert_eq!(
            page.entries[1].current_version_label().as_deref(),
            Some("Version 3")
        );
    }

    #[test]
    fn labels_the_version_of_a_draft() {
        let entry = |version| SpecListEntry {
            id: String::new(),
            title: String::new(),
            abbreviation: String::new(),
            status: SpecStatus::Published,
            current_version: version,
            created_by: String::new(),
            created_at: String::new(),
        };

        assert_eq!(
            entry(Some(0)).current_version_label().unwrap(),
            "No version yet"
        );
        assert_eq!(entry(Some(2)).current_version_label().unwrap(), "Version 2");
    }

    #[test]
    fn the_last_page_has_no_next_page_token() {
        let page = SpecListPage::from(ListSpecsResponse::default());

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
        let service = SpecListService::new(Arc::new(spec_gateway));

        let page = service
            .list_page(&parse("q=bank"), Some("token".into()))
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
        let service = SpecListService::new(Arc::new(spec_gateway));

        let result = service.list_page(&ListQuery::default(), None).await;

        assert!(matches!(result, Err(SpecGatewayError::Status(_))));
    }
}
