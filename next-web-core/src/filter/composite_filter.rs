//! A filter that runs a list of delegate filters as a single filter.

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

/// A generic composite [`HttpFilter`] that delegates to a chain (list) of
/// user-supplied filters, achieving the behavior of an [`HttpFilterChain`] while
/// only requiring [`HttpFilter`] instances.
///
/// This is useful for filters that need dependency injection, and can therefore
/// be set up in an application context: the composite itself is registered on
/// the container as a single filter, and it forwards to the configured
/// delegates.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// use next_web_core::filter::CompositeFilter;
/// use next_web_core::traits::filter::HttpFilter;
///
/// let mut composite = CompositeFilter::default();
/// composite.set_filters(Vec::<Arc<dyn HttpFilter>>::new());
/// assert!(composite.filters().is_empty());
/// ```
#[derive(Clone, Default)]
pub struct CompositeFilter {
    /// The delegate filters, in execution order.
    filters: Arc<[Arc<dyn HttpFilter>]>,
}

impl CompositeFilter {
    /// Sets the delegate filters, executed in the order supplied.
    ///
    /// The list is shared, not copied, so replacing it is cheap.
    pub fn set_filters<I>(&mut self, filters: I)
    where
        I: IntoIterator<Item = Arc<dyn HttpFilter>>,
    {
        self.filters = filters.into_iter().collect::<Arc<[_]>>();
    }

    /// Returns the delegate filters in execution order.
    #[must_use]
    pub fn filters(&self) -> &[Arc<dyn HttpFilter>] {
        &self.filters
    }
}

#[async_trait]
impl HttpFilter for CompositeFilter {
    /// Runs the delegate filters supplied with [`set_filters`](Self::set_filters)
    /// in order, and then delegates to the chain that invoked the composite.
    ///
    /// Each delegate receives a virtual chain; calling it runs the next
    /// delegate, and returning without calling it stops the composite (the
    /// original chain is then not reached).
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        VirtualFilterChain::new(Arc::clone(&self.filters), filter_chain)
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
///
/// The original chain is owned as a clone (`clone_box`) rather than borrowed, so
/// this type is `'static` and can implement [`HttpFilterChain`]. The position is
/// shared by clones, matching [`ApplicationFilterChain`].
///
/// [`ApplicationFilterChain`]: crate::filter::application_filter_chain::ApplicationFilterChain
struct VirtualFilterChain {
    /// The delegate filters of the composite, executed in order.
    additional_filters: Arc<[Arc<dyn HttpFilter>]>,
    /// The chain to delegate to once every delegate filter has run.
    original_chain: Box<dyn HttpFilterChain>,
    /// The index of the next delegate to run. Shared across clones.
    position: AtomicUsize,
}

impl VirtualFilterChain {
    /// Creates a chain that runs `additional_filters` and then continues with a
    /// clone of `original_chain`.
    fn new(
        additional_filters: Arc<[Arc<dyn HttpFilter>]>,
        original_chain: &dyn HttpFilterChain,
    ) -> Self {
        Self {
            additional_filters,
            original_chain: crate::clone_box(original_chain),
            position: AtomicUsize::new(0),
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
        let index = self.position.fetch_add(1, Ordering::Relaxed);

        match self.additional_filters.get(index) {
            Some(next_filter) => next_filter.do_filter(request, response, self).await,
            None => self.original_chain.do_filter(request, response).await,
        }
    }
}

impl Clone for VirtualFilterChain {
    fn clone(&self) -> Self {
        Self {
            additional_filters: Arc::clone(&self.additional_filters),
            original_chain: crate::clone_box(&*self.original_chain),
            position: AtomicUsize::new(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use axum::{body::Body, extract::Request, response::Response};

    use super::CompositeFilter;
    use crate::{
        filter::FilterError,
        traits::{
            filter::{HttpFilter, HttpFilterChain},
            http::{http_request::HttpRequest, http_response::HttpResponse},
            named::Named,
        },
    };

    /// A filter that records its name and then continues the chain.
    #[derive(Clone)]
    struct RecordingFilter {
        name: String,
        log: Arc<Mutex<Vec<String>>>,
    }

    impl Named for RecordingFilter {
        fn name(&self) -> &str {
            &self.name
        }
    }

    #[async_trait]
    impl HttpFilter for RecordingFilter {
        async fn do_filter(
            &self,
            request: &mut dyn HttpRequest,
            response: &mut dyn HttpResponse,
            filter_chain: &dyn HttpFilterChain,
        ) -> Result<(), FilterError> {
            self.log.lock().unwrap().push(self.name.clone());
            filter_chain.do_filter(request, response).await
        }
    }

    /// A terminal chain that records that it was reached.
    #[derive(Clone)]
    struct RecordingChain {
        log: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl HttpFilterChain for RecordingChain {
        async fn do_filter(
            &self,
            _request: &mut dyn HttpRequest,
            _response: &mut dyn HttpResponse,
        ) -> Result<(), FilterError> {
            self.log.lock().unwrap().push("original".to_string());
            Ok(())
        }
    }

    fn recording_filter(name: &str, log: &Arc<Mutex<Vec<String>>>) -> Arc<dyn HttpFilter> {
        Arc::new(RecordingFilter {
            name: name.to_string(),
            log: Arc::clone(log),
        })
    }

    #[tokio::test]
    async fn delegates_run_in_order_then_the_original_chain() {
        let log = Arc::new(Mutex::new(Vec::new()));

        let mut composite = CompositeFilter::default();
        composite.set_filters([
            recording_filter("first", &log),
            recording_filter("second", &log),
        ]);

        let mut request = Request::builder().body(Body::empty()).unwrap();
        let mut response = Response::builder().body(Body::empty()).unwrap();
        let chain = RecordingChain {
            log: Arc::clone(&log),
        };

        composite
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert_eq!(*log.lock().unwrap(), vec!["first", "second", "original"]);
    }

    #[tokio::test]
    async fn an_empty_composite_delegates_directly_to_the_original_chain() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let composite = CompositeFilter::default();

        let mut request = Request::builder().body(Body::empty()).unwrap();
        let mut response = Response::builder().body(Body::empty()).unwrap();
        let chain = RecordingChain {
            log: Arc::clone(&log),
        };

        composite
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert_eq!(*log.lock().unwrap(), vec!["original"]);
    }

    /// A filter that records its name and stops the chain.
    #[derive(Clone)]
    struct StoppingFilter {
        name: String,
        log: Arc<Mutex<Vec<String>>>,
    }

    impl Named for StoppingFilter {
        fn name(&self) -> &str {
            &self.name
        }
    }

    #[async_trait]
    impl HttpFilter for StoppingFilter {
        async fn do_filter(
            &self,
            _request: &mut dyn HttpRequest,
            _response: &mut dyn HttpResponse,
            _filter_chain: &dyn HttpFilterChain,
        ) -> Result<(), FilterError> {
            self.log.lock().unwrap().push(self.name.clone());
            Ok(())
        }
    }

    #[tokio::test]
    async fn a_stopping_delegate_prevents_the_original_chain() {
        let log = Arc::new(Mutex::new(Vec::new()));

        let mut composite = CompositeFilter::default();
        composite.set_filters([
            Arc::new(StoppingFilter {
                name: "stop".to_string(),
                log: Arc::clone(&log),
            }) as Arc<dyn HttpFilter>,
            recording_filter("never", &log),
        ]);

        let mut request = Request::builder().body(Body::empty()).unwrap();
        let mut response = Response::builder().body(Body::empty()).unwrap();
        let chain = RecordingChain {
            log: Arc::clone(&log),
        };

        composite
            .do_filter(&mut request, &mut response, &chain)
            .await
            .unwrap();

        assert_eq!(*log.lock().unwrap(), vec!["stop"]);
    }
}
