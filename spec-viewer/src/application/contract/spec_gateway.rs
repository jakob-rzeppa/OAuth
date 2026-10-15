use async_trait::async_trait;
use spec_api::{ListSpecsRequest, ListSpecsResponse, get_spec_response::Kind};

#[derive(Debug, thiserror::Error)]
pub enum SpecGatewayError {
    /// Includes a spec-provider that can't be reached, as `UNAVAILABLE`.
    #[error("spec-provider returned an error: {0}")]
    Status(#[from] tonic::Status),
    /// A `get_spec` answer with neither spec nor draft. The spec-provider answers an unknown id
    /// with `NOT_FOUND`, so this is a broken answer.
    #[error("spec-provider returned neither spec nor draft")]
    MissingKind,
}

/// The spec-provider, which knows the published specs and the drafts.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SpecGateway: Send + Sync {
    /// Fetches the published spec or draft with the given id.
    async fn get_spec(&self, id: &str) -> Result<Kind, SpecGatewayError>;

    /// Fetches one page of published specs and/or drafts.
    ///
    /// To get the next page, send the same request with the `next_page_token` of the response
    /// as its `page_token`.
    async fn list_specs(
        &self,
        request: ListSpecsRequest,
    ) -> Result<ListSpecsResponse, SpecGatewayError>;
}
