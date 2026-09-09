use std::sync::Arc;

use next_web_core::traits::http::http_request::HttpRequest;

use crate::web::authentication::Identity;

pub trait AuthenticationDetailsSource
where
    Self: Send + Sync,
{
    fn build_details(&self, context: &dyn HttpRequest) -> Arc<dyn Identity>;
}
