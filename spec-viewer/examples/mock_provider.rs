//! Standalone mock of the spec-provider for developing the spec-viewer by hand.
//!
//! Run with `cargo run --example mock_provider`, then start the spec-viewer with
//! `SPEC_PROVIDER_URL=http://localhost:50051` (the default in `.env`).
//!
//! Override the listen address with `MOCK_PROVIDER_ADDR`.
//!
//! `list_specs` serves a fixed catalogue: the two known specs below plus generated ones, so
//! filtering, searching, ordering and paging can be tried by hand. The generated ones are
//! `NOT_FOUND` for `get_spec`.

use prost_types::Timestamp;
use spec_api::{
    Draft, DraftOverview, DraftVisibility, GetSpecRequest, GetSpecResponse, ListSpecsRequest,
    ListSpecsResponse, Patch, Proposal, ProposalStatus, SortDirection, Spec, SpecKind,
    SpecListItem, SpecOverview, Version,
    get_spec_response::Kind,
    spec_list_item::Kind as ItemKind,
    spec_provider_server::{SpecProvider, SpecProviderServer},
};
use tonic::{Request, Response, Status};

// Known ids, anything else is NOT_FOUND.
/// A published spec.
const PUBLISHED_ID: &str = "b790c2e4-235e-49ce-b209-9755abd42538";
/// A draft with versions, an unversioned patch and proposals.
const DRAFT_ID: &str = "3f8a6d1e-5c42-4b7a-9e0d-2a1c7f6b8e34";
/// A response with neither spec nor draft.
const EMPTY_ID: &str = "c5d2e9a7-18b3-4f60-a4d1-7e93b0c2f815";

const DEFAULT_ADDR: &str = "127.0.0.1:50051";

/// The page size of `list_specs` when the request has none, and its upper limit.
const DEFAULT_PAGE_SIZE: usize = 50;
const MAX_PAGE_SIZE: usize = 100;
/// How many generated specs and drafts `list_specs` serves besides the known ones.
const GENERATED_COUNT: i64 = 30;

struct MockProvider;

#[tonic::async_trait]
impl SpecProvider for MockProvider {
    async fn get_spec(
        &self,
        request: Request<GetSpecRequest>,
    ) -> Result<Response<GetSpecResponse>, Status> {
        let id = request.into_inner().id;
        println!("get_spec({id})");

        let kind = match id.as_str() {
            PUBLISHED_ID => Some(Kind::Published(published_spec(id))),
            DRAFT_ID => Some(Kind::Draft(draft_spec(id))),
            EMPTY_ID => None,
            _ => return Err(Status::not_found("no such spec")),
        };
        Ok(Response::new(GetSpecResponse { kind }))
    }

    async fn list_specs(
        &self,
        request: Request<ListSpecsRequest>,
    ) -> Result<Response<ListSpecsResponse>, Status> {
        let request = request.into_inner();
        println!(
            "list_specs(kind={:?}, search={:?}, page_size={}, page_token={:?})",
            request.filter.as_ref().map(|filter| filter.kind),
            request.search,
            request.page_size,
            request.page_token,
        );

        list_specs(request).map(Response::new)
    }
}

/// Filters, searches, orders and pages the catalogue like the real spec-provider does.
///
/// The `page_token` is just the offset of the next page; the real ones are opaque.
fn list_specs(request: ListSpecsRequest) -> Result<ListSpecsResponse, Status> {
    let kind = SpecKind::try_from(request.filter.unwrap_or_default().kind)
        .map_err(|_| Status::invalid_argument("unknown spec kind"))?;
    let direction = SortDirection::try_from(request.order.unwrap_or_default().direction)
        .map_err(|_| Status::invalid_argument("unknown sort direction"))?;
    let offset: usize = match request.page_token.as_str() {
        "" => 0,
        token => token
            .parse()
            .map_err(|_| Status::invalid_argument("invalid page token"))?,
    };
    let page_size = match request.page_size {
        0 => DEFAULT_PAGE_SIZE,
        page_size => (page_size as usize).min(MAX_PAGE_SIZE),
    };
    let search = request.search.trim().to_lowercase();

    let mut items: Vec<ItemKind> = catalogue()
        .into_iter()
        .filter(|item| {
            !matches!(
                (item, kind),
                (ItemKind::Published(_), SpecKind::Draft)
                    | (ItemKind::Draft(_), SpecKind::Published)
            )
        })
        .filter(|item| {
            let (title, abbreviation, _) = fields(item);
            title.to_lowercase().contains(&search) || abbreviation.to_lowercase().contains(&search)
        })
        .collect();

    items.sort_by_key(|item| fields(item).2);
    if direction != SortDirection::Asc {
        items.reverse();
    }

    let next_offset = offset + page_size;
    let next_page_token = if next_offset < items.len() {
        next_offset.to_string()
    } else {
        String::new()
    };
    Ok(ListSpecsResponse {
        items: items
            .into_iter()
            .skip(offset)
            .take(page_size)
            .map(|kind| SpecListItem { kind: Some(kind) })
            .collect(),
        next_page_token,
    })
}

/// The known specs, which `get_spec` serves too, and generated ones to page through.
fn catalogue() -> Vec<ItemKind> {
    let mut items = vec![
        ItemKind::Published(SpecOverview {
            id: PUBLISHED_ID.into(),
            title: "Example Spec".into(),
            abbreviation: "EX".into(),
            created_by: "alice".into(),
            created_at: Some(timestamp(1_700_000_000)),
        }),
        ItemKind::Draft(DraftOverview {
            id: DRAFT_ID.into(),
            title: "Example Draft".into(),
            abbreviation: "EXD".into(),
            visibility: DraftVisibility::Public as i32,
            current_version: 1,
            created_by: "bob".into(),
            created_at: Some(timestamp(1_705_000_000)),
        }),
    ];

    let visibilities = [
        DraftVisibility::Public,
        DraftVisibility::Private,
        DraftVisibility::Hidden,
    ];
    for i in 1..=GENERATED_COUNT {
        let id = format!("00000000-0000-4000-8000-{i:012}");
        let created_at = Some(timestamp(1_710_000_000 + i * 86_400));
        items.push(if i % 2 == 0 {
            ItemKind::Published(SpecOverview {
                id,
                title: format!("Generated Spec {i}"),
                abbreviation: format!("GS{i}"),
                created_by: "alice".into(),
                created_at,
            })
        } else {
            ItemKind::Draft(DraftOverview {
                id,
                title: format!("Generated Draft {i}"),
                abbreviation: format!("GD{i}"),
                visibility: visibilities[i as usize % visibilities.len()] as i32,
                current_version: (i % 4) as u32,
                created_by: "bob".into(),
                created_at,
            })
        });
    }
    items
}

/// The title, abbreviation and creation time (in seconds) of a list item.
fn fields(item: &ItemKind) -> (&str, &str, i64) {
    let (title, abbreviation, created_at) = match item {
        ItemKind::Published(spec) => (&spec.title, &spec.abbreviation, &spec.created_at),
        ItemKind::Draft(draft) => (&draft.title, &draft.abbreviation, &draft.created_at),
    };
    (
        title,
        abbreviation,
        created_at
            .as_ref()
            .map_or(0, |created_at| created_at.seconds),
    )
}

fn published_spec(id: String) -> Spec {
    Spec {
        id,
        title: "Example Spec".into(),
        abbreviation: "EX".into(),
        versions: vec![
            Version {
                id: "v1".into(),
                patches: vec![accepted_patch("patch-1", 1_700_000_000)],
            },
            Version {
                id: "v2".into(),
                patches: vec![
                    accepted_patch("patch-2", 1_710_000_000),
                    accepted_patch("patch-3", 1_720_000_000),
                ],
            },
        ],
    }
}

fn draft_spec(id: String) -> Draft {
    Draft {
        id,
        title: "Example Draft".into(),
        abbreviation: "EXD".into(),
        visibility: DraftVisibility::Public as i32,
        versions: vec![Version {
            id: "v1".into(),
            patches: vec![accepted_patch("patch-1", 1_700_000_000)],
        }],
        unversioned_patches: vec![accepted_patch("patch-2", 1_710_000_000)],
        proposals: vec![
            Proposal {
                id: "proposal-1".into(),
                proposed_by: "alice".into(),
                created_at: Some(timestamp(1_715_000_000)),
                last_updated_at: Some(timestamp(1_716_000_000)),
                status: ProposalStatus::Draft as i32,
                content_uri: "https://example.com/proposals/1".into(),
            },
            Proposal {
                id: "proposal-2".into(),
                proposed_by: "bob".into(),
                created_at: Some(timestamp(1_717_000_000)),
                last_updated_at: Some(timestamp(1_718_000_000)),
                status: ProposalStatus::Ready as i32,
                content_uri: "https://example.com/proposals/2".into(),
            },
        ],
    }
}

fn accepted_patch(id: &str, seconds: i64) -> Patch {
    Patch {
        id: id.into(),
        proposed_by: "alice".into(),
        created_at: Some(timestamp(seconds)),
        accepted_by: "bob".into(),
        accepted_at: Some(timestamp(seconds + 3_600)),
        content_uri: format!("https://example.com/patches/{id}"),
    }
}

fn timestamp(seconds: i64) -> Timestamp {
    Timestamp { seconds, nanos: 0 }
}

#[tokio::main]
async fn main() {
    let addr = std::env::var("MOCK_PROVIDER_ADDR")
        .unwrap_or_else(|_| DEFAULT_ADDR.into())
        .parse()
        .expect("MOCK_PROVIDER_ADDR must be a socket address like 127.0.0.1:50051");

    println!("mock spec-provider listening on {addr}");
    if let Err(error) = tonic::transport::Server::builder()
        .add_service(SpecProviderServer::new(MockProvider))
        .serve(addr)
        .await
    {
        eprintln!("mock spec-provider stopped unexpectedly: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles(response: &ListSpecsResponse) -> Vec<&str> {
        response
            .items
            .iter()
            .map(|item| fields(item.kind.as_ref().unwrap()).0)
            .collect()
    }

    fn request() -> ListSpecsRequest {
        ListSpecsRequest::default()
    }

    #[test]
    fn lists_the_newest_first_by_default() {
        let response = list_specs(request()).unwrap();

        let created_at: Vec<i64> = response
            .items
            .iter()
            .map(|item| fields(item.kind.as_ref().unwrap()).2)
            .collect();
        assert_eq!(response.items.len(), GENERATED_COUNT as usize + 2);
        assert!(created_at.windows(2).all(|pair| pair[0] >= pair[1]));
    }

    #[test]
    fn lists_the_oldest_first_when_ascending() {
        let response = list_specs(ListSpecsRequest {
            order: Some(spec_api::ListSpecsOrder {
                direction: SortDirection::Asc.into(),
                ..Default::default()
            }),
            ..request()
        })
        .unwrap();

        assert_eq!(titles(&response)[0], "Example Spec");
    }

    #[test]
    fn filters_by_kind() {
        let published = list_specs(ListSpecsRequest {
            filter: Some(spec_api::ListSpecsFilter {
                kind: SpecKind::Published.into(),
            }),
            ..request()
        })
        .unwrap();
        let drafts = list_specs(ListSpecsRequest {
            filter: Some(spec_api::ListSpecsFilter {
                kind: SpecKind::Draft.into(),
            }),
            ..request()
        })
        .unwrap();

        assert!(
            published
                .items
                .iter()
                .all(|item| matches!(item.kind, Some(ItemKind::Published(_))))
        );
        assert!(
            drafts
                .items
                .iter()
                .all(|item| matches!(item.kind, Some(ItemKind::Draft(_))))
        );
        assert_eq!(
            published.items.len() + drafts.items.len(),
            GENERATED_COUNT as usize + 2
        );
    }

    #[test]
    fn searches_title_and_abbreviation_ignoring_case() {
        let by_title = list_specs(ListSpecsRequest {
            search: " EXAMPLE sp ".into(),
            ..request()
        })
        .unwrap();
        let by_abbreviation = list_specs(ListSpecsRequest {
            search: "exd".into(),
            ..request()
        })
        .unwrap();

        assert_eq!(titles(&by_title), ["Example Spec"]);
        assert_eq!(titles(&by_abbreviation), ["Example Draft"]);
    }

    #[test]
    fn pages_through_the_whole_catalogue() {
        let mut seen = Vec::new();
        let mut page_token = String::new();
        loop {
            let response = list_specs(ListSpecsRequest {
                page_size: 7,
                page_token,
                ..request()
            })
            .unwrap();
            assert!(response.items.len() <= 7);
            seen.extend(titles(&response).into_iter().map(String::from));
            page_token = response.next_page_token;
            if page_token.is_empty() {
                break;
            }
        }

        let all = list_specs(request()).unwrap();
        assert_eq!(seen, titles(&all));
    }

    #[test]
    fn rejects_an_invalid_page_token_and_unknown_enum_values() {
        let token = list_specs(ListSpecsRequest {
            page_token: "garbage".into(),
            ..request()
        });
        let kind = list_specs(ListSpecsRequest {
            filter: Some(spec_api::ListSpecsFilter { kind: 99 }),
            ..request()
        });
        let direction = list_specs(ListSpecsRequest {
            order: Some(spec_api::ListSpecsOrder {
                field: 0,
                direction: 99,
            }),
            ..request()
        });

        for result in [token, kind, direction] {
            assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
        }
    }
}
