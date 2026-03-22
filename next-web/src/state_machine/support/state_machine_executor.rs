use crate::state_machine::{
    state::StateMachineState, state_context::StateContext,
    support::state_machine_interceptor::StateMachineInterceptor,
    transition::StateMachineTransition,
};
use next_web_core::{async_trait, traits::message::Message};
use std::sync::Arc;

/// Interface for a StateMachine event executor.
#[async_trait]
pub trait StateMachineExecutor<S, E>
where
    Self: Send + Sync,
{
    /// Queue event.
    ///
    /// # Arguments
    /// * `message` - The message to queue
    /// * `callback` - The executor callback
    ///
    /// # Returns
    /// Completion when event is queued
    async fn queue_event(
        &self,
        message: Box<dyn Message<E>>,
        callback: Arc<dyn StateMachineExecutorCallback>,
    );

    /// Queue deferred event.
    ///
    /// # Arguments
    /// * `message` - The message to queue
    fn queue_deferred_event(&self, message: Box<dyn Message<E>>);

    /// Execute and check all triggerless transitions.
    ///
    /// # Arguments
    /// * `context` - The state context
    /// * `state` - The state
    ///
    /// # Returns
    /// Completion when handled
    async fn execute_triggerless_transitions(
        &self,
        context: &dyn StateContext<S, E>,
        state: &dyn StateMachineState<S, E>,
    );

    /// Sets if initial stage is enabled.
    ///
    /// # Arguments
    /// * `enabled` - The new flag
    fn set_initial_enabled(&self, enabled: bool);

    /// Set initial forwarded event.
    ///
    /// # Arguments
    /// * `message` - The forwarded message
    fn set_forwarded_initial_event(&self, message: Box<dyn Message<E>>);

    /// Sets the state machine executor transit handler.
    ///
    /// # Arguments
    /// * `state_machine_executor_transit` - The state machine executor transit handler
    fn set_state_machine_executor_transit(
        &mut self,
        transit_handler: Arc<dyn StateMachineExecutorTransit<S, E>>,
    );

    /// Adds a state machine interceptor.
    ///
    /// # Arguments
    /// * `interceptor` - The interceptor
    fn add_state_machine_interceptor(
        &mut self,
        interceptor: Arc<dyn StateMachineInterceptor<S, E>>,
    );
}

/// Callback interface when executor wants to handle transit.
#[async_trait]
pub trait StateMachineExecutorTransit<S, E>
where
    Self: Send + Sync,
{
    /// Called when executor wants to do a transit.
    ///
    /// # Arguments
    /// * `transition` - The transition
    /// * `state_context` - The state context
    /// * `message` - The message
    ///
    /// # Returns
    /// Completion when handled
    async fn transit(
        &self,
        transition: Arc<dyn StateMachineTransition<S, E>>,
        state_context: Arc<dyn StateContext<S, E>>,
        message: Box<dyn Message<E>>,
    );
}

/// Completion callback to notify back complete or error.
pub trait StateMachineExecutorCallback: Send + Sync {
    fn complete(&mut self);
    fn error(&mut self, error: Box<dyn std::error::Error + Send + Sync>);
}

/// Implementation of StateMachineExecutorCallback that uses a oneshot channel
pub struct DefaultStateMachineExecutorCallback {
    complete: Option<bool>,
    error: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl DefaultStateMachineExecutorCallback {
    pub fn accept(&self) {
        if self.complete.as_ref().map(|s| *s).unwrap_or_default() {
            todo!()
        }
    }
}

impl StateMachineExecutorCallback for DefaultStateMachineExecutorCallback {
    fn complete(&mut self) {
        let _ = self.complete.replace(true);
    }

    fn error(&mut self, error: Box<dyn std::error::Error + Send + Sync>) {
        let _ = self.error.replace(error);
    }
}

pub struct ExecutorErrorHolder(pub Box<dyn std::error::Error + Send + Sync>);

impl ExecutorErrorHolder {
    pub fn set_error(&mut self, error: Box<dyn std::error::Error + Send + Sync>) {
        self.0 = error;
    }

    pub fn get_error(&self) -> &Box<dyn std::error::Error + Send + Sync> {
        &self.0
    }
}
