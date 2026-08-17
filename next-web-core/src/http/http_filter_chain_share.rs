use async_trait::async_trait;

use crate::{
    filter::FilterError,
    traits::{
        filter::HttpFilterChain,
        http::{http_request::HttpRequest, http_response::HttpResponse},
    },
};

#[derive(Clone)]
pub struct HttpFilterChainShare {}

#[async_trait]
impl HttpFilterChain for HttpFilterChainShare {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        todo!()
    }
}

impl From<&dyn HttpFilterChain> for HttpFilterChainShare {
    fn from(value: &dyn HttpFilterChain) -> Self {
        HttpFilterChainShare {}
    }
}
