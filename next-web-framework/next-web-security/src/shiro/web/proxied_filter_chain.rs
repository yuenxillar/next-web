use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

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
    index: Arc<AtomicUsize>,
}

impl ProxiedFilterChain {
    pub fn new(orig: Box<dyn HttpFilterChain>, filters: Vec<Box<dyn HttpFilter>>) -> Self {
        Self {
            orig,
            filters,
            index: Arc::new(AtomicUsize::default()),
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
        println!("index: {}", self.index.load(Ordering::Relaxed));

        if self.filters.len() == self.index.load(Ordering::Relaxed) {
            self.orig.do_filter(request, response).await
        } else {
            let index = self.index.fetch_add(1, Ordering::Relaxed);

            if let Some(filter) = self.filters.get(index) {
                return filter.do_filter(request, response, self).await;
            }

            Ok(())
        }
    }
}
