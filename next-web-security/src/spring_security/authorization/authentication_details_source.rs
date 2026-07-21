use std::{any::Any, sync::Arc};

use next_web_core::traits::http::http_request::HttpRequest;

pub trait AuthenticationDetailsSource
where
    Self: Send + Sync,
{
    fn build_details(&self, context: &dyn HttpRequest) -> Arc<dyn Any + Send + Sync>;
}
