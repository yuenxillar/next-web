use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;

use crate::{
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
    },
};

#[derive(Clone, Default)]
pub struct ApplicationFilterChain {
    pos: Arc<AtomicUsize>,
    n: Arc<AtomicUsize>,
    filters: Vec<Arc<dyn HttpFilter>>,
}

impl ApplicationFilterChain {
    pub fn new(filters: Vec<Arc<dyn HttpFilter>>) -> Self {
        Self {
            pos: Arc::new(AtomicUsize::new(0)),
            n: Arc::new(AtomicUsize::new(filters.len())),
            filters,
        }
    }
}

#[async_trait]
impl HttpFilterChain for ApplicationFilterChain {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        // Call the next filter if there is one
        let pos = self.pos.load(Ordering::Relaxed);
        if pos < self.n.load(Ordering::Relaxed) {
            let filter = &self.filters[pos];
            filter.do_filter(request, response, self).await?;
            self.pos.store(pos + 1, Ordering::Relaxed);
        }

        Ok(())
    }
}
