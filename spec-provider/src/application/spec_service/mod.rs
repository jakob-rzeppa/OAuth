use std::sync::Arc;

use crate::application::contract::{
    draft_repository::DraftRepository, spec_repository::SpecRepository,
};

pub mod get;
pub mod list;

/// Use cases around specs and their drafts.
pub struct SpecService {
    spec_repository: Arc<dyn SpecRepository>,
    draft_repository: Arc<dyn DraftRepository>,
}

impl SpecService {
    pub fn new(
        spec_repository: Arc<dyn SpecRepository>,
        draft_repository: Arc<dyn DraftRepository>,
    ) -> SpecService {
        SpecService {
            spec_repository,
            draft_repository,
        }
    }
}
