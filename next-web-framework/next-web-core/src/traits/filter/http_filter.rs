use async_trait::async_trait;

use crate::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

#[async_trait]
pub trait HttpFilter
where
    Self: Send + Sync,
{
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), String>;
}
