use spec_api::{GetSpecRequest, get_spec_response::Kind, spec_provider_client::SpecProviderClient};
use tonic::transport::Channel;

use crate::spec::CLIENT;

#[derive(Debug, thiserror::Error)]
pub enum GetSpecError {
    /// Includes a spec-provider that can't be reached, as `UNAVAILABLE`.
    #[error("spec-provider returned an error: {0}")]
    Status(#[source] tonic::Status),
    #[error("spec-provider returned neither spec nor draft")]
    NotFound,
}

/// Fetches the published spec or draft with the given id from the configured spec-provider.
pub async fn get_spec(id: &str) -> Result<Kind, GetSpecError> {
    get_spec_with(CLIENT.clone(), id).await
}

async fn get_spec_with(
    mut client: SpecProviderClient<Channel>,
    id: &str,
) -> Result<Kind, GetSpecError> {
    client
        .get_spec(GetSpecRequest { id: id.to_string() })
        .await
        .map_err(GetSpecError::Status)?
        .into_inner()
        .kind
        .ok_or(GetSpecError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::lazy_client;
    use spec_api::{
        Draft, GetSpecResponse, ListSpecsRequest, ListSpecsResponse, Spec,
        spec_provider_server::{SpecProvider, SpecProviderServer},
    };
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::{Request, Response, Status};

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
            _: Request<ListSpecsRequest>,
        ) -> Result<Response<ListSpecsResponse>, Status> {
            Err(Status::unimplemented("list_specs"))
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

    #[tokio::test]
    async fn returns_published_spec() {
        let client = start_provider().await;
        match get_spec_with(client, "published").await.unwrap() {
            Kind::Published(spec) => assert_eq!(spec.title, "Title"),
            Kind::Draft(_) => panic!("expected published spec"),
        }
    }

    #[tokio::test]
    async fn returns_draft() {
        let client = start_provider().await;
        assert!(
            matches!(get_spec_with(client, "draft").await, Ok(Kind::Draft(d)) if d.id == "draft")
        );
    }

    #[tokio::test]
    async fn maps_not_found_to_status_error() {
        let client = start_provider().await;
        match get_spec_with(client, "missing").await {
            Err(GetSpecError::Status(status)) => assert_eq!(status.code(), tonic::Code::NotFound),
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[tokio::test]
    async fn rejects_response_without_kind() {
        let client = start_provider().await;
        assert!(matches!(
            get_spec_with(client, "empty").await,
            Err(GetSpecError::NotFound)
        ));
    }

    #[tokio::test]
    async fn maps_an_unreachable_provider_to_unavailable() {
        match get_spec_with(lazy_client("http://127.0.0.1:1"), "x").await {
            Err(GetSpecError::Status(status)) => {
                assert_eq!(status.code(), tonic::Code::Unavailable)
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }
}
