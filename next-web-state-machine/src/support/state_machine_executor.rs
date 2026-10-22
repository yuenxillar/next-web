//! The executor processes events and triggerless transitions of a state machine.

use std::sync::Arc;

use next_web_core::{async_trait, error::BoxError, messaging::Message};

use crate::state::StateMachineState;
use crate::state_context::StateContext;
use crate::support::state_machine_interceptor::StateMachineInterceptor;
use crate::transition::StateMachineTransition;

/// Interface for a state machine event executor.
#[async_trait]
pub trait StateMachineExecutor<S, E>: Send + Sync
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Queues an event for processing.
    ///
    /// # Arguments
    /// * `message` - the message to queue
    /// * `callback` - the callback notified when processing finished
    ///
    /// # Returns
    /// `true` when a transition happened, `false` when no transition matched
    async fn queue_event(
        &self,
        message: Arc<dyn Message<E>>,
        callback: Arc<dyn StateMachineExecutorCallback>,
    ) -> Result<bool, BoxError>;

    /// Queues a deferred event. Deferred events are replayed the next time an
    /// event is queued.
    ///
    /// # Arguments
    /// * `message` - the message to defer
    fn queue_deferred_event(&self, message: Arc<dyn Message<E>>);

    /// Executes and checks all triggerless transitions.
    ///
    /// # Arguments
    /// * `context` - the state context
    /// * `state` - the state to start from
    async fn execute_triggerless_transitions(
        &self,
        context: &dyn StateContext<S, E>,
        state: &dyn StateMachineState<S, E>,
    ) -> Result<(), BoxError>;

    /// Sets whether the initial transition is enabled.
    ///
    /// # Arguments
    /// * `enabled` - the new flag
    fn set_initial_enabled(&self, enabled: bool);

    /// Arms the triggers of the machine.
    ///
    /// A timer trigger only fires while it is armed, so the machine arms them
    /// when it starts.
    fn start_triggers(&self) {}

    /// Disarms the triggers of the machine.
    fn disarm_triggers(&self) {}
    /// Disarms the triggers of the machine.
    fn stop_triggers(&self) {}

    /// Sets the event forwarded to the initial transition.
    ///
    /// # Arguments
    /// * `message` - the forwarded message
    fn set_forwarded_initial_event(&self, message: Arc<dyn Message<E>>);

    /// Sets the handler which performs the actual state change.
    ///
    /// # Arguments
    /// * `transit_handler` - the transit handler
    fn set_state_machine_executor_transit(
        &self,
        transit_handler: Arc<dyn StateMachineExecutorTransit<S, E>>,
    );

    /// Adds a state machine interceptor.
    ///
    /// # Arguments
    /// * `interceptor` - the interceptor
    fn add_state_machine_interceptor(&self, interceptor: Arc<dyn StateMachineInterceptor<S, E>>);
}

/// Callback interface used when the executor wants to perform a transit.
#[async_trait]
pub trait StateMachineExecutorTransit<S, E>: Send + Sync {
    /// Called when the executor wants to perform a transit.
    ///
    /// The machine builds its own state context, so the executor only has to
    /// name the transition and the message to apply.
    ///
    /// # Arguments
    /// * `transition` - the transition to apply
    /// * `message` - the message which caused the transition
    ///
    /// # Returns
    /// `true` if the transition happened
    async fn transit(
        &self,
        transition: Arc<dyn StateMachineTransition<S, E>>,
        message: Option<Arc<dyn Message<E>>>,
    ) -> Result<bool, BoxError>;
}

/// Completion callback reporting success or a failure of event processing.
pub trait StateMachineExecutorCallback: Send + Sync {
    /// Called when processing completed successfully.
    fn complete(&self);

    /// Called when processing failed.
    ///
    /// # Arguments
    /// * `error` - the failure
    fn error(&self, error: BoxError);
}

/// A callback that records the outcome, so a caller can await the completion of
/// an event that was queued.
pub struct DefaultStateMachineExecutorCallback {
    inner: std::sync::Mutex<CallbackOutcome>,
}

#[derive(Default)]
struct CallbackOutcome {
    completed: bool,
    error: Option<String>,
}

impl DefaultStateMachineExecutorCallback {
    /// Creates a new callback.
    pub fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(CallbackOutcome::default()),
        }
    }

    /// Checks whether the callback was completed successfully.
    pub fn is_completed(&self) -> bool {
        crate::support::lock_util::lock(&self.inner).completed
    }

    /// Gets the recorded error message, if any.
    pub fn error_message(&self) -> Option<String> {
        crate::support::lock_util::lock(&self.inner).error.clone()
    }
}

impl Default for DefaultStateMachineExecutorCallback {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachineExecutorCallback for DefaultStateMachineExecutorCallback {
    fn complete(&self) {
        crate::support::lock_util::lock(&self.inner).completed = true;
    }

    fn error(&self, error: BoxError) {
        crate::support::lock_util::lock(&self.inner).error = Some(error.to_string());
    }
}
