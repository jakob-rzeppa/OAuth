use async_trait::async_trait;
use spec_api::{
    GetSpecRequest, ListSpecsRequest, ListSpecsResponse, get_spec_response::Kind,
    spec_provider_client::SpecProviderClient,
};
use tonic::transport::{Channel, Endpoint};

use crate::application::contract::spec_gateway::{GetSpecError, ListSpecsError, SpecGateway};

#[derive(Debug, thiserror::Error)]
#[error("SPEC_PROVIDER_URL is not a valid uri: {0}")]
pub struct InvalidSpecProviderUrl(#[source] tonic::transport::Error);

/// The spec-provider, reached over gRPC.
pub struct GrpcSpecGateway {
    client: SpecProviderClient<Channel>,
}

impl GrpcSpecGateway {
    /// The connection is established with the first call, and again after it drops. Calls
    /// share it.
    ///
    /// Must be called within the tokio runtime.
    pub fn connect_lazy(endpoint: &str) -> Result<Self, InvalidSpecProviderUrl> {
        let channel = Endpoint::from_shared(endpoint.to_string())
            .map_err(InvalidSpecProviderUrl)?
            .connect_lazy();
        Ok(GrpcSpecGateway {
            client: SpecProviderClient::new(channel),
        })
    }
}

#[async_trait]
impl SpecGateway for GrpcSpecGateway {
    async fn get_spec(&self, id: &str) -> Result<Kind, GetSpecError> {
        self.client
            .clone()
            .get_spec(GetSpecRequest { id: id.to_string() })
            .await
            .map_err(GetSpecError::Status)?
            .into_inner()
            .kind
            .ok_or(GetSpecError::NotFound)
    }

    async fn list_specs(
        &self,
        request: ListSpecsRequest,
    ) -> Result<ListSpecsResponse, ListSpecsError> {
        self.client
            .clone()
            .list_specs(request)
            .await
            .map(tonic::Response::into_inner)
            .map_err(ListSpecsError::Status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spec_api::{
        Draft, GetSpecResponse, ListSpecsFilter, ListSpecsOrder, SortDirection, Spec, SpecKind,
        SpecListItem, SpecOverview,
        spec_list_item::Kind as ItemKind,
        spec_provider_server::{SpecProvider, SpecProviderServer},
    };
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::{Request, Response, Status};

    /// `get_spec` answers by the `id` of the request:
    /// - `"published"` and `"draft"` are an empty spec or draft with that id,
    /// - `"empty"` is a response with neither,
    /// - anything else is `NOT_FOUND`.
    ///
    /// `list_specs` answers by the `search` of the request:
    /// - `"fail"` is an `INVALID_ARGUMENT` error,
    /// - `"none"` is an empty page,
    /// - anything else is one item titled with the debug output of the whole request,
    ///   and a `next_page_token` unless the request already had a `page_token`.
    struct FakeProvider;

    #[tonic::async_trait]
    impl SpecProvider for FakeProvider {
        async fn get_spec(
            &self,
            request: Request<GetSpecRequest>,
        ) -> Result<Response<GetSpecResponse>, Status> {
            let id = request.into_inner().id;
            let kind = match id.as_str() {
                "published" => Some(Kind::Published(Spec {
                    id,
                    title: "Title".into(),
                    ..Default::default()
                })),
                "draft" => Some(Kind::Draft(Draft {
                    id,
                    ..Default::default()
                })),
                "empty" => None,
                _ => return Err(Status::not_found("no such spec")),
            };
            Ok(Response::new(GetSpecResponse { kind }))
        }

        async fn list_specs(
            &self,
            request: Request<ListSpecsRequest>,
        ) -> Result<Response<ListSpecsResponse>, Status> {
            let request = request.into_inner();
            match request.search.as_str() {
                "fail" => Err(Status::invalid_argument("bad search")),
                "none" => Ok(Response::new(ListSpecsResponse::default())),
                _ => {
                    let next_page_token = if request.page_token.is_empty() {
                        "next".to_string()
                    } else {
                        String::new()
                    };
                    Ok(Response::new(ListSpecsResponse {
                        items: vec![SpecListItem {
                            kind: Some(ItemKind::Published(SpecOverview {
                                title: format!("{request:?}"),
                                ..Default::default()
                            })),
                        }],
                        next_page_token,
                    }))
                }
            }
        }
    }

    async fn start_provider() -> GrpcSpecGateway {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(
            tonic::transport::Server::builder()
                .add_service(SpecProviderServer::new(FakeProvider))
                .serve_with_incoming(TcpListenerStream::new(listener)),
        );
        GrpcSpecGateway::connect_lazy(&format!("http://{addr}")).unwrap()
    }

    fn unreachable_provider() -> GrpcSpecGateway {
        GrpcSpecGateway::connect_lazy("http://127.0.0.1:1").unwrap()
    }

    fn request(search: &str) -> ListSpecsRequest {
        ListSpecsRequest {
            search: search.into(),
            ..Default::default()
        }
    }

    fn first_title(response: &ListSpecsResponse) -> &str {
        match response.items[0].kind.as_ref().unwrap() {
            ItemKind::Published(spec) => &spec.title,
            ItemKind::Draft(draft) => &draft.title,
        }
    }

    #[tokio::test]
    async fn returns_published_spec() {
        let gateway = start_provider().await;

        match gateway.get_spec("published").await.unwrap() {
            Kind::Published(spec) => assert_eq!(spec.title, "Title"),
            Kind::Draft(_) => panic!("expected published spec"),
        }
    }

    #[tokio::test]
    async fn returns_draft() {
        let gateway = start_provider().await;

        assert!(matches!(gateway.get_spec("draft").await, Ok(Kind::Draft(d)) if d.id == "draft"));
    }

    #[tokio::test]
    async fn maps_not_found_to_status_error() {
        let gateway = start_provider().await;

        match gateway.get_spec("missing").await {
            Err(GetSpecError::Status(status)) => assert_eq!(status.code(), tonic::Code::NotFound),
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[tokio::test]
    async fn rejects_response_without_kind() {
        let gateway = start_provider().await;

        assert!(matches!(
            gateway.get_spec("empty").await,
            Err(GetSpecError::NotFound)
        ));
    }

    #[tokio::test]
    async fn maps_an_unreachable_provider_to_unavailable_on_get_spec() {
        match unreachable_provider().get_spec("x").await {
            Err(GetSpecError::Status(status)) => {
                assert_eq!(status.code(), tonic::Code::Unavailable)
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[tokio::test]
    async fn sends_every_field_of_the_request_to_the_provider() {
        let gateway = start_provider().await;
        let request = ListSpecsRequest {
            page_size: 7,
            page_token: "token".into(),
            filter: Some(ListSpecsFilter {
                kind: SpecKind::Draft.into(),
            }),
            search: "bank".into(),
            order: Some(ListSpecsOrder {
                direction: SortDirection::Asc.into(),
                ..Default::default()
            }),
        };

        let response = gateway.list_specs(request.clone()).await.unwrap();

        assert_eq!(first_title(&response), format!("{request:?}"));
    }

    #[tokio::test]
    async fn returns_the_items_and_the_next_page_token() {
        let gateway = start_provider().await;

        let response = gateway.list_specs(request("bank")).await.unwrap();

        assert_eq!(response.items.len(), 1);
        assert_eq!(response.next_page_token, "next");
    }

    #[tokio::test]
    async fn returns_an_empty_token_on_the_last_page() {
        let gateway = start_provider().await;
        let request = ListSpecsRequest {
            page_token: "next".into(),
            ..request("bank")
        };

        let response = gateway.list_specs(request).await.unwrap();

        assert!(response.next_page_token.is_empty());
    }

    #[tokio::test]
    async fn an_empty_page_is_not_an_error() {
        let gateway = start_provider().await;

        let response = gateway.list_specs(request("none")).await.unwrap();

        assert!(response.items.is_empty());
        assert!(response.next_page_token.is_empty());
    }

    #[tokio::test]
    async fn maps_a_provider_error_to_status_error() {
        let gateway = start_provider().await;

        match gateway.list_specs(request("fail")).await {
            Err(ListSpecsError::Status(status)) => {
                assert_eq!(status.code(), tonic::Code::InvalidArgument)
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[tokio::test]
    async fn maps_an_unreachable_provider_to_unavailable_on_list_specs() {
        match unreachable_provider().list_specs(request("bank")).await {
            Err(ListSpecsError::Status(status)) => {
                assert_eq!(status.code(), tonic::Code::Unavailable)
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }
}
