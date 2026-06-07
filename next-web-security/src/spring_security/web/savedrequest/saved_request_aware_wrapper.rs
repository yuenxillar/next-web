use std::sync::Arc;

use crate::web::savedrequest::SavedRequest;

#[derive(Clone)]
pub struct SavedRequestAwareWrapper {
    saved_request: Arc<dyn SavedRequest>,
}

impl SavedRequestAwareWrapper {
    pub fn new(saved_request: Arc<dyn SavedRequest>) -> Self {
        Self { saved_request }
    }
}
