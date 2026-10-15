use std::sync::Arc;

use crate::application::{
    contract::{content_gateway::ContentGateway, spec_gateway::SpecGateway},
    diff::DiffService,
    markdown::MarkdownService,
    spec_list::SpecListService,
    spec_view::SpecViewService,
    timestamp::TimestampService,
};

/// The services the handlers share.
#[derive(Clone)]
pub struct AppState {
    pub spec_view_service: Arc<SpecViewService>,
    pub spec_list_service: Arc<SpecListService>,
    // The handlers don't call these themselves yet; the use cases above hold their own copies.
    #[allow(dead_code)]
    pub diff_service: Arc<DiffService>,
    #[allow(dead_code)]
    pub markdown_service: Arc<MarkdownService>,
    #[allow(dead_code)]
    pub timestamp_service: Arc<TimestampService>,
}

impl AppState {
    /// Builds the services on top of the gateways.
    pub fn new(
        spec_gateway: Arc<dyn SpecGateway>,
        content_gateway: Arc<dyn ContentGateway>,
    ) -> Self {
        let diff_service = Arc::new(DiffService::new());
        let markdown_service = Arc::new(MarkdownService::new());
        let timestamp_service = Arc::new(TimestampService::new());

        AppState {
            spec_view_service: Arc::new(SpecViewService::new(
                spec_gateway.clone(),
                content_gateway,
                diff_service.clone(),
                markdown_service.clone(),
                timestamp_service.clone(),
            )),
            spec_list_service: Arc::new(SpecListService::new(
                spec_gateway,
                timestamp_service.clone(),
            )),
            diff_service,
            markdown_service,
            timestamp_service,
        }
    }
}
