//! An owned handle to the rest of a filter chain.
//!
//! A filter receives the remainder of its chain as `&dyn HttpFilterChain`, which
//! borrows it for the duration of the call. Some filters have to keep that
//! remainder for longer — a
//! [`ConcurrentSessionFilter`](https://docs.rs/next-web-security) stores it on an
//! event it publishes, so the chain can still be run when the session that was
//! being checked turns out to have expired. [`HttpFilterChainShare`] is that owned
//! handle: it clones the chain into a value that can be stored and sent, and
//! running it runs the chain it was made from.

use async_trait::async_trait;

use crate::http::{
    HttpRequest, HttpResponse,
    filter::{FilterError, HttpFilterChain},
};

/// An owned, cloneable handle to the rest of a filter chain.
///
/// Created with `HttpFilterChainShare::from(&*chain)` or
/// [`HttpFilterChainShare::new`], and run with
/// [`HttpFilterChain::do_filter`], which forwards to the chain it was made from.
///
/// The chain is cloned when the handle is created, and clones of the handle share
/// that one chain, so a position the chain tracks (see
/// [`HttpFilterChain`]) is shared with it rather than duplicated.
#[derive(Clone)]
pub struct HttpFilterChainShare {
    /// The remainder of the chain this handle stands for.
    chain: Box<dyn HttpFilterChain>,
}

impl HttpFilterChainShare {
    /// Takes an owned handle to `chain`.
    ///
    /// The chain is cloned, so the handle keeps working after the chain it was
    /// taken from has gone out of scope.
    #[must_use]
    pub fn new(chain: &dyn HttpFilterChain) -> Self {
        Self {
            chain: dyn_clone::clone_box(chain),
        }
    }
}

#[async_trait]
impl HttpFilterChain for HttpFilterChainShare {
    /// Runs the chain this handle was made from.
    ///
    /// # Errors
    ///
    /// Returns the [`FilterError`] produced by the next filter or by the terminal
    /// handler, exactly as running the original chain would.
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        self.chain.do_filter(request, response).await
    }
}

impl From<&dyn HttpFilterChain> for HttpFilterChainShare {
    fn from(value: &dyn HttpFilterChain) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use async_trait::async_trait;

    use super::HttpFilterChainShare;
    use crate::http::{
        HttpRequest, HttpResponse,
        filter::{FilterError, HttpFilterChain},
    };

    /// A chain that counts how often it was run.
    #[derive(Clone)]
    struct CountingChain {
        runs: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl HttpFilterChain for CountingChain {
        async fn do_filter(
            &self,
            _request: &mut dyn HttpRequest,
            _response: &mut dyn HttpResponse,
        ) -> Result<(), FilterError> {
            self.runs.fetch_add(1, Ordering::SeqCst);

            Ok(())
        }
    }

    #[tokio::test]
    async fn a_share_runs_the_chain_it_was_made_from() {
        let runs = Arc::new(AtomicUsize::new(0));
        let chain = CountingChain {
            runs: Arc::clone(&runs),
        };

        let share = HttpFilterChainShare::from(&chain as &dyn HttpFilterChain);
        // The handle outlives the chain it was taken from.
        drop(chain);

        let mut request = axum::extract::Request::builder()
            .body(axum::body::Body::empty())
            .expect("the request is built");
        let mut response = axum::response::Response::builder()
            .body(axum::body::Body::empty())
            .expect("the response is built");

        share
            .do_filter(&mut request, &mut response)
            .await
            .expect("the chain runs");

        assert_eq!(
            runs.load(Ordering::SeqCst),
            1,
            "the share forwards to the chain it was made from instead of doing nothing"
        );
    }
}
