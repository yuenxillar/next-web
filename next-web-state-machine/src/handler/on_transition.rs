//! Handlers which run when a transition matches a filter.
//!
//! A handler is the Rust counterpart of the `@OnTransition` methods of the Java
//! implementation: a piece of user code which runs when the machine takes a
//! transition from one state to another for a given event. It is plugged in as a
//! state machine interceptor, which is the hook which receives the
//! [`StateContext`].

use std::sync::Arc;

use next_web_core::error::BoxError;
use next_web_core::messaging::Message;

use crate::StateMachine;
use crate::state::StateMachineState;
use crate::state_context::StateContext;
use crate::support::state_machine_interceptor::StateMachineInterceptor;
use crate::transition::StateMachineTransition;

/// The action of a transition handler.
///
/// The interceptor hook which runs it is synchronous, so the action reports its
/// failures directly instead of returning a future. A handler which has to do
/// asynchronous work reads what it needs from the context and spawns the work
/// itself, which keeps the failure of the transition and the failure of the
/// background work separate.
pub type OnTransitionAction<S, E> =
    Arc<dyn Fn(&dyn StateContext<S, E>) -> Result<(), BoxError> + Send + Sync>;

/// Runs an action when a transition matches a source, a target or an event.
///
/// Every condition which is set has to match, so a handler without conditions
/// runs for every transition.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct OnTransition<S, E> {
    source: Option<S>,
    target: Option<S>,
    event: Option<E>,
    action: OnTransitionAction<S, E>,
}

impl<S, E> OnTransition<S, E>
where
    S: Clone + PartialEq + Send + Sync + 'static,
    E: Clone + PartialEq + Send + Sync + 'static,
{
    /// Creates a handler which runs the given action.
    ///
    /// # Arguments
    /// * `action` - the action which runs for a matching transition
    pub fn new(action: OnTransitionAction<S, E>) -> Self {
        Self {
            source: None,
            target: None,
            event: None,
            action,
        }
    }

    /// Restricts the handler to transitions which leave the given state.
    ///
    /// # Arguments
    /// * `source` - the source state
    pub fn source(mut self, source: S) -> Self {
        self.source = Some(source);
        self
    }

    /// Restricts the handler to transitions which enter the given state.
    ///
    /// # Arguments
    /// * `target` - the target state
    pub fn target(mut self, target: S) -> Self {
        self.target = Some(target);
        self
    }

    /// Restricts the handler to transitions which are triggered by the event.
    ///
    /// # Arguments
    /// * `event` - the event
    pub fn event(mut self, event: E) -> Self {
        self.event = Some(event);
        self
    }

    /// Checks whether the transition of the context matches the handler.
    ///
    /// # Arguments
    /// * `context` - the context of the transition
    pub fn matches(&self, context: &dyn StateContext<S, E>) -> bool {
        if let Some(source) = self.source.as_ref() {
            let leaves_source = context
                .source()
                .map(|state| state.contains_id(source))
                .unwrap_or(false);

            if !leaves_source {
                return false;
            }
        }

        if let Some(target) = self.target.as_ref() {
            let enters_target = context
                .target()
                .map(|state| state.contains_id(target))
                .unwrap_or(false);

            if !enters_target {
                return false;
            }
        }

        if let Some(event) = self.event.as_ref() {
            let matches_event = context.event().map(|current| current == event).unwrap_or(false);

            if !matches_event {
                return false;
            }
        }

        true
    }
}

#[next_web_core::async_trait]
impl<S, E> StateMachineInterceptor<S, E> for OnTransition<S, E>
where
    S: Clone + PartialEq + Send + Sync + 'static,
    E: Clone + PartialEq + Send + Sync + 'static,
{
    fn pre_event(
        &self,
        message: Arc<dyn Message<E>>,
        _state_machine: &dyn StateMachine<S, E>,
    ) -> Result<Arc<dyn Message<E>>, BoxError> {
        Ok(message)
    }

    fn pre_state_change(
        &self,
        _state: &dyn StateMachineState<S, E>,
        _message: Option<&dyn Message<E>>,
        _transition: Option<&dyn StateMachineTransition<S, E>>,
        _state_machine: &dyn StateMachine<S, E>,
        _root_state_machine: &dyn StateMachine<S, E>,
    ) -> Result<(), BoxError> {
        Ok(())
    }

    fn post_state_change(
        &self,
        _state: &dyn StateMachineState<S, E>,
        _message: Option<&dyn Message<E>>,
        _transition: Option<&dyn StateMachineTransition<S, E>>,
        _state_machine: &dyn StateMachine<S, E>,
        _root_state_machine: &dyn StateMachine<S, E>,
    ) {
    }

    fn pre_transition(&self, _state_context: &dyn StateContext<S, E>) -> Result<(), BoxError> {
        Ok(())
    }

    fn post_transition(&self, state_context: &dyn StateContext<S, E>) -> Result<(), BoxError> {
        if state_context.transition().is_none() {
            return Ok(());
        }

        if !self.matches(state_context) {
            return Ok(());
        }

        let action = self.action.clone();
        action(state_context)
    }

    fn state_machine_error(
        &self,
        _state_machine: &dyn StateMachine<S, E>,
        error: BoxError,
    ) -> Option<BoxError> {
        Some(error)
    }
}
