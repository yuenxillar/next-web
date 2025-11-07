use async_trait::async_trait;

use crate::{
    error::BoxError,
    traits::{
        filter::http_filter_chain::HttpFilterChain,
        http::{http_request::HttpRequest, http_response::HttpResponse},
    },
};

#[derive(Clone, Default)]
pub struct ApplicationFilterChain {}

#[async_trait]
impl HttpFilterChain for ApplicationFilterChain {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        Ok(())
    }
}
