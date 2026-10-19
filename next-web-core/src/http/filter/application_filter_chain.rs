//! The standard [`HttpFilterChain`] over a fixed list of filters.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;

use crate::{
    http::filter::{FilterError, HttpFilter, HttpFilterChain},
    http::{HttpRequest, HttpResponse},
};

/// Runs a fixed list of [`HttpFilter`]s in order.
///
/// Each call to [`HttpFilterChain::do_filter`] runs the next filter, so a filter
/// continues the chain by calling the chain it was given. Once the last filter
/// has run, further calls simply succeed.
///
/// # Shared position
///
/// The position lives in an [`AtomicUsize`] shared by every clone. That matters
/// when a filter clones the remainder of the chain (as
/// [`CompositeFilter`](crate::filter::CompositeFilter) does): the clone resumes
/// from the current position instead of restarting the chain.
///
/// The chain is exhausted when the position passes the last filter, and it then
/// rewinds itself so it can serve a later request. A chain still serves one
/// request at a time; use [`ApplicationFilterChain::reset`] between requests if
/// a filter can stop the chain early.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// use next_web_core::http::filter::{ApplicationFilterChain, HttpFilter};
///
/// let filters: Arc<[Arc<dyn HttpFilter>]> = Arc::from(Vec::new());
/// let chain = ApplicationFilterChain::new(filters);
/// assert_eq!(chain.position(), 0);
/// ```
#[derive(Clone, Default)]
pub struct ApplicationFilterChain {
    /// The filters, executed in order.
    filters: Arc<[Arc<dyn HttpFilter>]>,
    /// The index of the next filter to run. Shared across clones.
    position: Arc<AtomicUsize>,
}

impl ApplicationFilterChain {
    /// Creates a chain that runs `filters` in order.
    #[must_use]
    pub fn new(filters: Arc<[Arc<dyn HttpFilter>]>) -> Self {
        Self {
            filters,
            position: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Returns the filters of the chain in execution order.
    #[must_use]
    pub fn filters(&self) -> &[Arc<dyn HttpFilter>] {
        &self.filters
    }

    /// Returns the index of the next filter to run.
    ///
    /// A value equal to the number of filters means the chain is exhausted.
    #[must_use]
    pub fn position(&self) -> usize {
        self.position.load(Ordering::Acquire)
    }

    /// Rewinds the chain to its first filter.
    ///
    /// Because the position is shared by every clone, this rewinds all of them.
    pub fn reset(&self) {
        self.position.store(0, Ordering::Release);
    }
}

#[async_trait]
impl HttpFilterChain for ApplicationFilterChain {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        let index = self.position.fetch_add(1, Ordering::AcqRel);

        match self.filters.get(index) {
            // Run the next filter and let it continue the chain. The filter
            // receives `self`, so it resumes from the position just advanced.
            Some(filter) => filter.do_filter(request, response, self).await,
            // The chain is exhausted: rewind so it can serve another request.
            None => {
                self.position.store(0, Ordering::Release);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use axum::{body::Body, extract::Request, response::Response};

    use super::ApplicationFilterChain;
    use crate::{
        Named,
        http::{
            HttpRequest, HttpResponse,
            filter::{FilterError, HttpFilter, HttpFilterChain},
        },
    };

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

    fn recording_filter(name: &str, log: &Arc<Mutex<Vec<String>>>) -> Arc<dyn HttpFilter> {
        Arc::new(RecordingFilter {
            name: name.to_string(),
            log: Arc::clone(log),
        })
    }

    /// A filter that records itself and stops the chain without continuing it.
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
    async fn filters_run_in_order_and_the_chain_rewinds() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let filters: Arc<[Arc<dyn HttpFilter>]> = Arc::from(vec![
            recording_filter("first", &log),
            recording_filter("second", &log),
        ]);
        let chain = ApplicationFilterChain::new(filters);

        let mut request = Request::builder().body(Body::empty()).unwrap();
        let mut response = Response::builder().body(Body::empty()).unwrap();

        chain.do_filter(&mut request, &mut response).await.unwrap();

        assert_eq!(*log.lock().unwrap(), vec!["first", "second"]);
        assert_eq!(chain.position(), 0, "the chain rewinds when exhausted");
    }

    #[tokio::test]
    async fn reset_rewinds_a_chain_that_was_stopped_early() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let filters: Arc<[Arc<dyn HttpFilter>]> = Arc::from(vec![Arc::new(StoppingFilter {
            name: "only".to_string(),
            log: Arc::clone(&log),
        })
            as Arc<dyn HttpFilter>]);
        let chain = ApplicationFilterChain::new(filters);

        let mut request = Request::builder().body(Body::empty()).unwrap();
        let mut response = Response::builder().body(Body::empty()).unwrap();
        chain.do_filter(&mut request, &mut response).await.unwrap();

        // The filter stopped the chain, so it did not rewind itself.
        assert_eq!(chain.position(), 1);

        chain.reset();
        assert_eq!(chain.position(), 0);
    }
}
