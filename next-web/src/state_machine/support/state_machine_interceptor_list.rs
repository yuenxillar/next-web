use std::sync::Arc;

use next_web_core::traits::message::Message;
use tracing::debug;

use crate::state_machine::state::StateMachineState;
use crate::state_machine::state_context::StateContext;
use crate::state_machine::support::state_machine_interceptor::StateMachineInterceptor;
use crate::state_machine::transition::StateMachineTransition;
use crate::state_machine::StateMachine;

/// A list of state machine interceptors that can be applied in order.
///
/// This struct manages a collection of interceptors and provides methods
/// to apply them at various points in the state machine lifecycle.
pub struct StateMachineInterceptorList<S, E> {
    interceptors: Vec<Arc<dyn StateMachineInterceptor<S, E>>>,
}

impl<S, E> StateMachineInterceptorList<S, E> {
    /// Creates a new empty interceptor list.
    pub fn new() -> Self {
        Self {
            interceptors: Default::default(),
        }
    }

    /// Sets the interceptors, clearing any existing interceptors.
    ///
    /// # Arguments
    /// * `interceptors` - The list of interceptors to set
    ///
    /// # Returns
    /// `true` if the interceptor list changed as a result of the call
    pub fn set(&mut self, interceptors: Vec<Arc<dyn StateMachineInterceptor<S, E>>>) {
        self.interceptors.clear();
        self.interceptors.extend(interceptors);
    }

    /// Adds an interceptor to the list.
    ///
    /// # Arguments
    /// * `interceptor` - The interceptor to add
    ///
    /// # Returns
    /// `true` (as specified by `Collection::add`)
    pub fn add(&mut self, interceptor: Arc<dyn StateMachineInterceptor<S, E>>) {
        self.interceptors.push(interceptor);
    }

    /// Removes an interceptor from the list.
    ///
    /// # Arguments
    /// * `interceptor` - The interceptor to remove
    ///
    /// # Returns
    /// `true` if the interceptor was found and removed
    pub fn remove(&mut self, interceptor: Arc<dyn StateMachineInterceptor<S, E>>) {
        self.interceptors.retain(|i| !Arc::ptr_eq(i, &interceptor));
    }

    /// Pre-processes an event before it is handled by the state machine.
    ///
    /// Interceptors are applied in order. If any interceptor returns `None`,
    /// processing stops and `None` is returned.
    ///
    /// # Arguments
    /// * `message` - The message containing the event
    /// * `state_machine` - The state machine instance
    ///
    /// # Returns
    /// The (possibly modified) message, or `None` if processing was stopped
    pub fn pre_event<'a>(
        &self,
        message: &'a mut dyn Message<E>,
        state_machine: &dyn StateMachine<S, E>,
    ) -> &'a mut dyn Message<E> {
        for interceptor in self.interceptors.iter() {
            match interceptor.pre_event(message, state_machine) {
                Ok(_) => {}
                Err(error) => {
                    debug!("Interceptor stopped event processing, case: {:?}", error);
                    break;
                }
            }
        }

        message
    }

    /// Pre-processes a state change before it occurs.
    ///
    /// # Arguments
    /// * `state` - The target state
    /// * `message` - The message that triggered the change
    /// * `transition` - The transition being performed
    /// * `state_machine` - The state machine instance
    /// * `root_state_machine` - The root state machine instance
    pub fn pre_state_change(
        &self,
        state: &dyn StateMachineState<S, E>,
        message: &dyn Message<E>,
        transition: &dyn StateMachineTransition<S, E>,
        state_machine: &dyn StateMachine<S, E>,
        root_state_machine: &dyn StateMachine<S, E>,
    ) {
        for interceptor in self.interceptors.iter() {
            interceptor.pre_state_change(
                state,
                message,
                transition,
                state_machine,
                root_state_machine,
            );
        }
    }

    /// Post-processes a state change after it has occurred.
    ///
    /// # Arguments
    /// * `state` - The new state
    /// * `message` - The message that triggered the change
    /// * `transition` - The transition that was performed
    /// * `state_machine` - The state machine instance
    /// * `root_state_machine` - The root state machine instance
    pub fn post_state_change(
        &self,
        state: &dyn StateMachineState<S, E>,
        message: &dyn Message<E>,
        transition: &dyn StateMachineTransition<S, E>,
        state_machine: &dyn StateMachine<S, E>,
        root_state_machine: &dyn StateMachine<S, E>,
    ) {
        for interceptor in self.interceptors.iter() {
            interceptor.post_state_change(
                state,
                message,
                transition,
                state_machine,
                root_state_machine,
            );
        }
    }

    /// Pre-processes a transition before it occurs.
    ///
    /// Interceptors are applied in order. If any interceptor returns `None`,
    /// processing stops and `None` is returned.
    ///
    /// # Arguments
    /// * `state_context` - The state context
    ///
    /// # Returns
    /// The (possibly modified) state context, or `None` if processing was stopped
    pub async fn pre_transition<'a>(
        &self,
        state_context: &'a dyn StateContext<S, E>,
    ) -> &'a dyn StateContext<S, E> {
        for interceptor in self.interceptors.iter() {
            match interceptor.pre_transition(state_context) {
                Ok(_) => {}
                Err(error) => {
                    debug!(
                        "Interceptor stopped transition processing, case: {:?}",
                        error
                    );
                    break;
                }
            }
        }

        state_context
    }

    /// Post-processes a transition after it has occurred.
    ///
    /// Interceptors are applied in order. If any interceptor returns `None`,
    /// processing stops and `None` is returned.
    ///
    /// # Arguments
    /// * `state_context` - The state context
    ///
    /// # Returns
    /// The (possibly modified) state context, or `None` if processing was stopped
    pub fn post_transition<'a>(
        &self,
        state_context: &'a dyn StateContext<S, E>,
    ) -> &'a dyn StateContext<S, E> {
        for interceptor in self.interceptors.iter() {
            match interceptor.post_transition(state_context) {
                Ok(_) => {}
                Err(error) => {
                    debug!(
                        "Interceptor stopped transition processing, case: {:?}",
                        error
                    );
                    break;
                }
            }
        }

        state_context
    }

    /// Handles an error that occurred in the state machine.
    ///
    /// Interceptors are applied in order. If any interceptor returns `None`,
    /// processing stops and `None` is returned, effectively suppressing the error.
    ///
    /// # Arguments
    /// * `state_machine` - The state machine instance
    /// * `error` - The error that occurred
    ///
    /// # Returns
    /// The (possibly modified) error, or `None` if the error was handled/suppressed
    pub fn state_machine_error(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        error: Box<dyn std::error::Error>,
    ) -> Box<dyn std::error::Error> {
        let mut e = error;
        for interceptor in self.interceptors.iter() {
            e = interceptor.state_machine_error(state_machine, e);
        }

        e
    }
}

impl<S, E> Clone for StateMachineInterceptorList<S, E>
where
    S: Clone,
    E: Clone,
{
    fn clone(&self) -> Self {
        Self {
            interceptors: self.interceptors.clone(),
        }
    }
}

impl<S, E> Default for StateMachineInterceptorList<S, E> {
    fn default() -> Self {
        Self {
            interceptors: Default::default(),
        }
    }
}
