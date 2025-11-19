use next_web_core::{
    async_trait,
    error::BoxError,
    traits::{
        filter::{http_filter::HttpFilter, http_filter_chain::HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
    },
};

#[derive(Clone)]
pub struct ProxiedFilterChain {
    orig: Box<dyn HttpFilterChain>,
    filters: Vec<Box<dyn HttpFilter>>,
    index: usize,
}

impl ProxiedFilterChain {
    pub fn new(orig: Box<dyn HttpFilterChain>, filters: Vec<Box<dyn HttpFilter>>) -> Self {
        Self {
            orig,
            filters,
            index: 0,
        }
    }
}

#[async_trait]
impl HttpFilterChain for ProxiedFilterChain {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        if self.filters.len() == self.index {
            self.orig.do_filter(request, response).await
        } else {
            if let Some(filter) = self.filters.get(self.index) {
                return filter.do_filter(request, response, self).await;
            }

            Ok(())
        }
    }
}
