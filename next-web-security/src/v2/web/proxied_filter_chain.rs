use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use next_web_core::{
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse,
    },
};

/// A chain that runs the filters of an inner chain in front of an outer one.
///
/// The position it reached is shared by the clones of the instance, so a filter
/// that hands the chain on as an owned value continues the chain where it was.
#[derive(Clone)]
pub struct ProxiedFilterChain {
    orig: Box<dyn HttpFilterChain>,
    filters: Vec<Box<dyn HttpFilter>>,
    index: Arc<AtomicUsize>,
}

impl ProxiedFilterChain {
    /// Creates a chain that runs `filters` and then `orig`.
    pub fn new(orig: Box<dyn HttpFilterChain>, filters: Vec<Box<dyn HttpFilter>>) -> Self {
        Self {
            orig,
            filters,
            index: Arc::new(AtomicUsize::default()),
        }
    }

    /// Returns the chain that runs after the filters of this one.
    pub fn orig(&self) -> &dyn HttpFilterChain {
        self.orig.as_ref()
    }

    /// Returns the filters of this chain, in the order they run.
    pub fn filters(&self) -> &[Box<dyn HttpFilter>] {
        &self.filters
    }
}

#[async_trait]
impl HttpFilterChain for ProxiedFilterChain {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
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
