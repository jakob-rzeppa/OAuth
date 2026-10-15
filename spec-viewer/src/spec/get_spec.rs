use spec_api::{GetSpecRequest, get_spec_response::Kind, spec_provider_client::SpecProviderClient};

use crate::config::Config;

#[derive(Debug, thiserror::Error)]
pub enum GetSpecError {
    #[error("failed to connect to spec-provider: {0}")]
    Connect(#[source] tonic::transport::Error),
    #[error("spec-provider returned an error: {0}")]
    Status(#[source] tonic::Status),
    #[error("spec-provider returned neither spec nor draft")]
    NotFound,
}

/// Fetches the published spec or draft with the given id from the configured spec-provider.
pub async fn get_spec(id: &str) -> Result<Kind, GetSpecError> {
    get_spec_at(Config::spec_provider_url(), id).await
}

async fn get_spec_at(endpoint: &str, id: &str) -> Result<Kind, GetSpecError> {
    let mut client = SpecProviderClient::connect(endpoint.to_string())
        .await
        .map_err(GetSpecError::Connect)?;

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

    async fn start_provider() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(
            tonic::transport::Server::builder()
                .add_service(SpecProviderServer::new(FakeProvider))
                .serve_with_incoming(TcpListenerStream::new(listener)),
        );
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn returns_published_spec() {
        let url = start_provider().await;
        match get_spec_at(&url, "published").await.unwrap() {
            Kind::Published(spec) => assert_eq!(spec.title, "Title"),
            Kind::Draft(_) => panic!("expected published spec"),
        }
    }

    #[tokio::test]
    async fn returns_draft() {
        let url = start_provider().await;
        assert!(matches!(get_spec_at(&url, "draft").await, Ok(Kind::Draft(d)) if d.id == "draft"));
    }

    #[tokio::test]
    async fn maps_not_found_to_status_error() {
        let url = start_provider().await;
        match get_spec_at(&url, "missing").await {
            Err(GetSpecError::Status(status)) => assert_eq!(status.code(), tonic::Code::NotFound),
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[tokio::test]
    async fn rejects_response_without_kind() {
        let url = start_provider().await;
        assert!(matches!(
            get_spec_at(&url, "empty").await,
            Err(GetSpecError::NotFound)
        ));
    }

    #[tokio::test]
    async fn fails_to_connect_to_unreachable_provider() {
        assert!(matches!(
            get_spec_at("http://127.0.0.1:1", "x").await,
            Err(GetSpecError::Connect(_))
        ));
    }
}
