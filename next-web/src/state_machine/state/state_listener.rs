use next_web_core::async_trait;

use crate::state_machine::state_context::StateContext;

/// `StateListener` for various state events.
///
/// This trait defines callbacks for monitoring state-specific events
/// such as entry, exit, and completion. Implementations can use these
/// callbacks for state-specific side effects, validation, or coordination.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
#[async_trait]
pub trait StateListener<S, E> {
    /// Called when `State` want to notify of its entry.
    ///
    /// # Arguments
    /// * `context` - the state context
    async fn on_entry(&self, context: &dyn StateContext<S, E>);

    /// Called when `State` want to notify of its exit.
    ///
    /// # Arguments
    /// * `context` - the state context
    async fn on_exit(&self, context: &dyn StateContext<S, E>);

    /// Called when `State` want to notify of its completion.
    ///
    /// # Arguments
    /// * `context` - the state context
    async fn on_complete(&self, context: &dyn StateContext<S, E>);

    /// Called when `State` want to notify of its completion.
    /// This method returns a future that completes when the listener
    /// has finished processing the completion event.
    ///
    /// # Arguments
    /// * `context` - the state context
    ///
    /// # Returns
    /// A future that completes when the listener has finished processing
    async fn do_on_complete(&self, context: &dyn StateContext<S, E>);
}
