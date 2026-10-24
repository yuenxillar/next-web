//! Helpers the end-to-end tests of the security filters share.
//!
//! A built chain is a list of filters, and the chain proxy runs them in order,
//! each one handing the request to the next. Running a chain that way is what an
//! end-to-end test does, and every end-to-end test needs the same runner — so it
//! lives here rather than in each configurer's tests.

use std::sync::Arc;

use next_web_core::{
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse,
    },
};

/// Runs the filters of a chain in order, the way the chain proxy does.
#[derive(Clone)]
pub(crate) struct Chain {
    filters: Arc<Vec<Arc<dyn HttpFilter>>>,
    index: usize,
}

impl Chain {
    /// Returns a chain that starts at the first filter.
    pub(crate) fn new(filters: Arc<Vec<Arc<dyn HttpFilter>>>) -> Self {
        Self { filters, index: 0 }
    }

    /// Runs the filter at the current index and everything after it.
    pub(crate) async fn run(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        HttpFilterChain::do_filter(self, request, response).await
    }
}

#[async_trait]
impl HttpFilterChain for Chain {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        let Some(filter) = self.filters.get(self.index) else {
            return Ok(());
        };
        let next = Self {
            filters: self.filters.clone(),
            index: self.index + 1,
        };

        filter.do_filter(request, response, &next).await
    }
}
