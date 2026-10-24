//! A no-op state listener, meant to be overridden selectively.

use next_web_core::async_trait;

use crate::state::state_listener::StateListener;
use crate::state_context::StateContext;

/// Adapter for [`StateListener`].
///
/// Every callback does nothing, so an implementation only has to override the
/// callbacks it cares about. This mirrors the Java `StateListenerAdapter`.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct StateListenerAdapter<S, E> {
    _marker: std::marker::PhantomData<(S, E)>,
}

impl<S, E> Default for StateListenerAdapter<S, E> {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S, E> StateListenerAdapter<S, E> {
    /// Creates a new adapter.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl<S, E> StateListener<S, E> for StateListenerAdapter<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    async fn on_entry(&self, _context: &dyn StateContext<S, E>) {}

    async fn on_exit(&self, _context: &dyn StateContext<S, E>) {}

    async fn on_complete(&self, _context: &dyn StateContext<S, E>) {}

    async fn do_on_complete(&self, _context: &dyn StateContext<S, E>) {}
}
