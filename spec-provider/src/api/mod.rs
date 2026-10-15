use spec_api::{
    GetSpecRequest, GetSpecResponse, ListSpecsRequest, ListSpecsResponse,
    spec_provider_server::SpecProvider,
};
use tonic::{Request, Response, Status};

use crate::application::spec_service::SpecService;

mod get_spec;
mod list_specs;
mod mapping;

/// The gRPC service. Each rpc delegates to its own module.
pub struct SpecProviderService {
    spec_service: SpecService,
}

impl SpecProviderService {
    pub fn new(spec_service: SpecService) -> SpecProviderService {
        SpecProviderService { spec_service }
    }
}

#[tonic::async_trait]
impl SpecProvider for SpecProviderService {
    async fn get_spec(
        &self,
        request: Request<GetSpecRequest>,
    ) -> Result<Response<GetSpecResponse>, Status> {
        self.get_spec_endpoint(request).await
    }

    async fn list_specs(
        &self,
        request: Request<ListSpecsRequest>,
    ) -> Result<Response<ListSpecsResponse>, Status> {
        self.list_specs_endpoint(request).await
    }
}
