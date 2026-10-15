use std::sync::Arc;

use crate::application::{spec_list::SpecListService, spec_view::SpecViewService};

/// The services the handlers share.
#[derive(Clone)]
pub struct AppState {
    pub spec_view_service: Arc<SpecViewService>,
    pub spec_list_service: Arc<SpecListService>,
}

impl AppState {
    pub fn new(spec_view_service: SpecViewService, spec_list_service: SpecListService) -> Self {
        AppState {
            spec_view_service: Arc::new(spec_view_service),
            spec_list_service: Arc::new(spec_list_service),
        }
    }
}
