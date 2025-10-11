use std::sync::Arc;

use axum::extract::Request;

use crate::{authorization::authorization_manager::AuthorizationManager, core::filter::Filter};

pub struct AuthorizationFilter {
    authorization_manager: Arc<dyn AuthorizationManager<Request>>,
    observe_once_per_request: bool,
    filter_error_dispatch: bool,
    filter_async_dispatch: bool,
}

impl AuthorizationFilter {
    pub fn new(
        authorization_manager: Arc<dyn AuthorizationManager<Request>>, 
    ) -> Self {
        Self {
            authorization_manager,
            observe_once_per_request: false,
            filter_error_dispatch: true,
            filter_async_dispatch: true
        }
    }
}


impl Filter for AuthorizationFilter {
    fn do_filter(&self, req: & Request, res: & axum::response::Response, next: axum::middleware::Next) {
        todo!()
    }
}