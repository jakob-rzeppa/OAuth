use spec_api::{ListSpecsOrder, ListSpecsRequest, ListSpecsResponse};
use tonic::{Request, Response, Status};

use crate::{
    api::SpecProviderService,
    application::{
        contract::{SortDirection, SpecOrder, SpecOrderField},
        spec_service::list::{ListSpecsError, ListSpecsFilter, ListSpecsParams, SpecKind},
    },
};

mod page_token;

impl SpecProviderService {
    pub(super) async fn list_specs_endpoint(
        &self,
        request: Request<ListSpecsRequest>,
    ) -> Result<Response<ListSpecsResponse>, Status> {
        let request = request.into_inner();

        let after = match request.page_token.as_str() {
            "" => None,
            token => Some(
                page_token::decode(token)
                    .ok_or_else(|| Status::invalid_argument("The page token is invalid."))?,
            ),
        };

        let params = ListSpecsParams {
            filter: ListSpecsFilter {
                kind: kind_from(request.filter.map(|filter| filter.kind).unwrap_or_default())?,
            },
            search: Some(request.search),
            order: order_from(request.order.unwrap_or_default())?,
            after,
            page_size: match request.page_size {
                0 => None,
                page_size => Some(page_size as usize),
            },
        };

        let page = self
            .spec_service
            .list_specs(params)
            .await
            .map_err(|e| match e {
                ListSpecsError::DatabaseError => {
                    Status::internal("An internal server error occurred.")
                }
            })?;

        Ok(Response::new(ListSpecsResponse {
            items: page.items.into_iter().map(Into::into).collect(),
            next_page_token: page
                .next
                .map(|cursor| page_token::encode(&cursor))
                .unwrap_or_default(),
        }))
    }
}

fn kind_from(kind: i32) -> Result<SpecKind, Status> {
    let kind = spec_api::SpecKind::try_from(kind)
        .map_err(|_| Status::invalid_argument("The spec kind filter is invalid."))?;

    Ok(match kind {
        spec_api::SpecKind::Unspecified | spec_api::SpecKind::Both => SpecKind::Both,
        spec_api::SpecKind::Published => SpecKind::Published,
        spec_api::SpecKind::Draft => SpecKind::Draft,
    })
}

fn order_from(order: ListSpecsOrder) -> Result<SpecOrder, Status> {
    let field = spec_api::ListSpecsOrderField::try_from(order.field)
        .map_err(|_| Status::invalid_argument("The order field is invalid."))?;
    let direction = spec_api::SortDirection::try_from(order.direction)
        .map_err(|_| Status::invalid_argument("The sort direction is invalid."))?;

    Ok(SpecOrder {
        field: match field {
            spec_api::ListSpecsOrderField::Unspecified
            | spec_api::ListSpecsOrderField::CreatedAt => SpecOrderField::CreatedAt,
        },
        direction: match direction {
            spec_api::SortDirection::Unspecified | spec_api::SortDirection::Desc => {
                SortDirection::Descending
            }
            spec_api::SortDirection::Asc => SortDirection::Ascending,
        },
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{TimeZone, Utc};
    use spec_api::{
        ListSpecsFilter, ListSpecsOrder, SortDirection, SpecKind, spec_list_item::Kind,
    };
    use tonic::Code;
    use uuid::Uuid;

    use super::*;
    use crate::{
        application::{
            contract::{draft_repository::DraftRepository, spec_repository::SpecRepository},
            spec_service::SpecService,
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
            DraftVisibility::Hidden,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Utc.timestamp_opt(created_at_secs, 0).unwrap(),
        )
    }

    async fn make_service(specs: &[Spec], drafts: &[Draft]) -> SpecProviderService {
        let spec_repository = Arc::new(InMemorySpecRepository::default());
        for spec in specs {
            spec_repository.register(spec).await.unwrap();
        }
        let draft_repository = Arc::new(InMemoryDraftRepository::default());
        for draft in drafts {
            draft_repository.register(draft).await.unwrap();
        }
        SpecProviderService::new(SpecService::new(spec_repository, draft_repository))
    }

    async fn list(service: &SpecProviderService, request: ListSpecsRequest) -> ListSpecsResponse {
        service
            .list_specs_endpoint(Request::new(request))
            .await
            .unwrap()
            .into_inner()
    }

    fn titles(response: &ListSpecsResponse) -> Vec<&str> {
        response
            .items
            .iter()
            .map(
                |item| match item.kind.as_ref().expect("item without a kind") {
                    Kind::Published(spec) => spec.title.as_str(),
                    Kind::Draft(draft) => draft.title.as_str(),
                },
            )
            .collect()
    }

    fn filter_of(kind: SpecKind) -> Option<ListSpecsFilter> {
        Some(ListSpecsFilter { kind: kind.into() })
    }

    #[tokio::test]
    async fn lists_specs_and_drafts_newest_first_by_default() {
        let service = make_service(
            &[make_spec_at("spec-10", "S", 10)],
            &[make_draft_at("draft-20", "D", 20)],
        )
        .await;

        let response = list(&service, ListSpecsRequest::default()).await;

        assert_eq!(titles(&response), ["draft-20", "spec-10"]);
        assert!(response.next_page_token.is_empty());
    }

    #[tokio::test]
    async fn lists_the_overview_message_of_each_kind() {
        let draft = make_draft_at("draft-20", "D", 20);
        let spec = make_spec_at("spec-10", "S", 10);
        let (draft_id, spec_id) = (draft.id().to_string(), spec.id().to_string());
        let service = make_service(&[spec], &[draft]).await;

        let response = list(&service, ListSpecsRequest::default()).await;

        let Some(Kind::Draft(draft)) = &response.items[0].kind else {
            panic!("expected a draft first");
        };
        assert_eq!(draft.id, draft_id);
        assert_eq!(draft.visibility, spec_api::DraftVisibility::Hidden as i32);
        let Some(Kind::Published(spec)) = &response.items[1].kind else {
            panic!("expected a published spec second");
        };
        assert_eq!(spec.id, spec_id);
    }

    #[tokio::test]
    async fn lists_only_published_specs_when_filtering_for_them() {
        let service = make_service(
            &[make_spec_at("spec-10", "S", 10)],
            &[make_draft_at("draft-20", "D", 20)],
        )
        .await;
        let request = ListSpecsRequest {
            filter: filter_of(SpecKind::Published),
            ..Default::default()
        };

        let response = list(&service, request).await;

        assert_eq!(titles(&response), ["spec-10"]);
    }

    #[tokio::test]
    async fn lists_only_drafts_when_filtering_for_them() {
        let service = make_service(
            &[make_spec_at("spec-10", "S", 10)],
            &[make_draft_at("draft-20", "D", 20)],
        )
        .await;
        let request = ListSpecsRequest {
            filter: filter_of(SpecKind::Draft),
            ..Default::default()
        };

        let response = list(&service, request).await;

        assert_eq!(titles(&response), ["draft-20"]);
    }

    #[tokio::test]
    async fn lists_both_kinds_when_filtering_for_both() {
        let service = make_service(
            &[make_spec_at("spec-10", "S", 10)],
            &[make_draft_at("draft-20", "D", 20)],
        )
        .await;
        let request = ListSpecsRequest {
            filter: filter_of(SpecKind::Both),
            ..Default::default()
        };

        let response = list(&service, request).await;

        assert_eq!(titles(&response), ["draft-20", "spec-10"]);
    }

    #[tokio::test]
    async fn lists_the_oldest_first_when_ordering_ascending() {
        let service = make_service(
            &[make_spec_at("spec-10", "S", 10)],
            &[make_draft_at("draft-20", "D", 20)],
        )
        .await;
        let request = ListSpecsRequest {
            order: Some(ListSpecsOrder {
                direction: SortDirection::Asc.into(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let response = list(&service, request).await;

        assert_eq!(titles(&response), ["spec-10", "draft-20"]);
    }

    #[tokio::test]
    async fn lists_the_newest_first_when_ordering_descending_by_created_at() {
        let service = make_service(
            &[make_spec_at("spec-10", "S", 10)],
            &[make_draft_at("draft-20", "D", 20)],
        )
        .await;
        let request = ListSpecsRequest {
            order: Some(ListSpecsOrder {
                field: spec_api::ListSpecsOrderField::CreatedAt.into(),
                direction: SortDirection::Desc.into(),
            }),
            ..Default::default()
        };

        let response = list(&service, request).await;

        assert_eq!(titles(&response), ["draft-20", "spec-10"]);
    }

    #[tokio::test]
    async fn lists_only_items_matching_the_search() {
        let service = make_service(
            &[make_spec_at("Open Banking", "OB", 10)],
            &[
                make_draft_at("Payments", "BANK", 20),
                make_draft_at("Identity", "ID", 30),
            ],
        )
        .await;
        let request = ListSpecsRequest {
            search: "bank".to_string(),
            ..Default::default()
        };

        let response = list(&service, request).await;

        assert_eq!(titles(&response), ["Payments", "Open Banking"]);
    }

    #[tokio::test]
    async fn pages_with_the_next_page_token() {
        let service = make_service(
            &[
                make_spec_at("spec-10", "S", 10),
                make_spec_at("spec-30", "S", 30),
            ],
            &[make_draft_at("draft-20", "D", 20)],
        )
        .await;

        let first = list(
            &service,
            ListSpecsRequest {
                page_size: 2,
                ..Default::default()
            },
        )
        .await;
        let second = list(
            &service,
            ListSpecsRequest {
                page_size: 2,
                page_token: first.next_page_token.clone(),
                ..Default::default()
            },
        )
        .await;

        assert_eq!(titles(&first), ["spec-30", "draft-20"]);
        assert!(!first.next_page_token.is_empty());
        assert_eq!(titles(&second), ["spec-10"]);
        assert!(second.next_page_token.is_empty());
    }

    #[tokio::test]
    async fn fails_with_invalid_argument_for_a_malformed_page_token() {
        let service = make_service(&[], &[]).await;
        let request = ListSpecsRequest {
            page_token: "garbage".to_string(),
            ..Default::default()
        };

        let status = service
            .list_specs_endpoint(Request::new(request))
            .await
            .unwrap_err();

        assert_eq!(status.code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn fails_with_invalid_argument_for_an_unknown_kind() {
        let service = make_service(&[], &[]).await;
        let request = ListSpecsRequest {
            filter: Some(ListSpecsFilter { kind: 99 }),
            ..Default::default()
        };

        let status = service
            .list_specs_endpoint(Request::new(request))
            .await
            .unwrap_err();

        assert_eq!(status.code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn fails_with_invalid_argument_for_an_unknown_order_field() {
        let service = make_service(&[], &[]).await;
        let request = ListSpecsRequest {
            order: Some(ListSpecsOrder {
                field: 99,
                direction: 0,
            }),
            ..Default::default()
        };

        let status = service
            .list_specs_endpoint(Request::new(request))
            .await
            .unwrap_err();

        assert_eq!(status.code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn fails_with_invalid_argument_for_an_unknown_sort_direction() {
        let service = make_service(&[], &[]).await;
        let request = ListSpecsRequest {
            order: Some(ListSpecsOrder {
                field: 0,
                direction: 99,
            }),
            ..Default::default()
        };

        let status = service
            .list_specs_endpoint(Request::new(request))
            .await
            .unwrap_err();

        assert_eq!(status.code(), Code::InvalidArgument);
    }
}
