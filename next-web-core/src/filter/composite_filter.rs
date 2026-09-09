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
        named::Named,
    },
};

/// A generic composite `HttpFilter` that just delegates its behavior to a chain
/// (list) of user-supplied filters, achieving the functionality of an
/// `HttpFilterChain`, but conveniently using only `HttpFilter` instances.
///
/// This is useful for filters that require dependency injection, and can therefore
/// be set up in an application context. Typically, this composite is used as a
/// single filter registered on a servlet container.
#[derive(Clone, Default)]
pub struct CompositeFilter {
    filters: Vec<Arc<dyn HttpFilter>>,
}

impl CompositeFilter {
    /// Sets the delegate filters, executed in the order supplied.
    pub fn set_filters(&mut self, filters: Vec<Arc<dyn HttpFilter>>) {
        self.filters = filters;
    }

    /// Returns the delegate filters in execution order.
    pub fn filters(&self) -> &[Arc<dyn HttpFilter>] {
        &self.filters
    }
}

#[async_trait]
impl HttpFilter for CompositeFilter {
    /// Forms a temporary chain from the list of delegate filters supplied
    /// ([`set_filters`](Self::set_filters)) and executes them in order. Each filter
    /// delegates to the next one in the list, achieving the normal behavior of an
    /// `HttpFilterChain`, despite the fact that this is an `HttpFilter`.
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        VirtualFilterChain::new(&self.filters, filter_chain)
            .do_filter(request, response)
            .await
    }
}

impl Named for CompositeFilter {
    fn name(&self) -> &str {
        "CompositeFilter"
    }
}

/// Internal chain that advances through the composite's delegate filters and
/// delegates to the original application chain after the last filter.
#[derive(Clone)]
struct VirtualFilterChain {
    original_chain: Box<dyn HttpFilterChain>,
    additional_filters: Arc<Vec<Arc<dyn HttpFilter>>>,
    current_position: Arc<AtomicUsize>,
}

impl VirtualFilterChain {
    fn new(
        additional_filters: &[Arc<dyn HttpFilter>],
        original_chain: &dyn HttpFilterChain,
    ) -> Self {
        Self {
            original_chain: crate::clone_box(original_chain),
            additional_filters: Arc::new(additional_filters.to_vec()),
            current_position: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl HttpFilterChain for VirtualFilterChain {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        let index = self.current_position.fetch_add(1, Ordering::AcqRel);
        if let Some(filter) = self.additional_filters.get(index) {
            filter.do_filter(request, response, self).await
        } else {
            self.original_chain.do_filter(request, response).await
        }
    }
}
