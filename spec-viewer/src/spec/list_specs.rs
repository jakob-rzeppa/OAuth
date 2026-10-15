use spec_api::{ListSpecsRequest, ListSpecsResponse, spec_provider_client::SpecProviderClient};
use tonic::transport::Channel;

use crate::spec::CLIENT;

#[derive(Debug, thiserror::Error)]
pub enum ListSpecsError {
    /// Includes a spec-provider that can't be reached, as `UNAVAILABLE`.
    #[error("spec-provider returned an error: {0}")]
    Status(#[source] tonic::Status),
}

/// Fetches one page of published specs and/or drafts from the configured spec-provider.
///
/// To get the next page, send the same request with the `next_page_token` of the response
/// as its `page_token`.
pub async fn list_specs(request: ListSpecsRequest) -> Result<ListSpecsResponse, ListSpecsError> {
    list_specs_with(CLIENT.clone(), request).await
}

async fn list_specs_with(
    mut client: SpecProviderClient<Channel>,
    request: ListSpecsRequest,
) -> Result<ListSpecsResponse, ListSpecsError> {
    client
        .list_specs(request)
        .await
        .map(tonic::Response::into_inner)
        .map_err(ListSpecsError::Status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::lazy_client;
    use spec_api::{
        GetSpecRequest, GetSpecResponse, ListSpecsFilter, ListSpecsOrder, SortDirection, SpecKind,
        SpecListItem, SpecOverview,
        spec_list_item::Kind,
        spec_provider_server::{SpecProvider, SpecProviderServer},
    };
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::{Request, Response, Status};

    /// Answers by the `search` of the request:
    /// - `"fail"` is an `INVALID_ARGUMENT` error,
    /// - `"none"` is an empty page,
    /// - anything else is one item titled with the debug output of the whole request,
    ///   and a `next_page_token` unless the request already had a `page_token`.
    struct FakeProvider;

    #[tonic::async_trait]
    impl SpecProvider for FakeProvider {
        async fn get_spec(
            &self,
            _: Request<GetSpecRequest>,
        ) -> Result<Response<GetSpecResponse>, Status> {
            Err(Status::unimplemented("get_spec"))
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
                            kind: Some(Kind::Published(SpecOverview {
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

    async fn start_provider() -> SpecProviderClient<Channel> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(
            tonic::transport::Server::builder()
                .add_service(SpecProviderServer::new(FakeProvider))
                .serve_with_incoming(TcpListenerStream::new(listener)),
        );
        lazy_client(&format!("http://{addr}"))
    }

    fn request(search: &str) -> ListSpecsRequest {
        ListSpecsRequest {
            search: search.into(),
            ..Default::default()
        }
    }

    fn first_title(response: &ListSpecsResponse) -> &str {
        match response.items[0].kind.as_ref().unwrap() {
            Kind::Published(spec) => &spec.title,
            Kind::Draft(draft) => &draft.title,
        }
    }

    #[tokio::test]
    async fn sends_every_field_of_the_request_to_the_provider() {
        let client = start_provider().await;
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

        let response = list_specs_with(client, request.clone()).await.unwrap();

        assert_eq!(first_title(&response), format!("{request:?}"));
    }

    #[tokio::test]
    async fn returns_the_items_and_the_next_page_token() {
        let client = start_provider().await;

        let response = list_specs_with(client, request("bank")).await.unwrap();

        assert_eq!(response.items.len(), 1);
        assert_eq!(response.next_page_token, "next");
    }

    #[tokio::test]
    async fn returns_an_empty_token_on_the_last_page() {
        let client = start_provider().await;
        let request = ListSpecsRequest {
            page_token: "next".into(),
            ..request("bank")
        };

        let response = list_specs_with(client, request).await.unwrap();

        assert!(response.next_page_token.is_empty());
    }

    #[tokio::test]
    async fn an_empty_page_is_not_an_error() {
        let client = start_provider().await;

        let response = list_specs_with(client, request("none")).await.unwrap();

        assert!(response.items.is_empty());
        assert!(response.next_page_token.is_empty());
    }

    #[tokio::test]
    async fn maps_a_provider_error_to_status_error() {
        let client = start_provider().await;

        match list_specs_with(client, request("fail")).await {
            Err(ListSpecsError::Status(status)) => {
                assert_eq!(status.code(), tonic::Code::InvalidArgument)
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[tokio::test]
    async fn maps_an_unreachable_provider_to_unavailable() {
        match list_specs_with(lazy_client("http://127.0.0.1:1"), request("bank")).await {
            Err(ListSpecsError::Status(status)) => {
                assert_eq!(status.code(), tonic::Code::Unavailable)
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }
}
