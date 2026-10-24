use std::sync::Arc;
use std::sync::RwLock;

use next_web_core::error::BoxError;
use next_web_core::messaging::Message;
use tracing::debug;

use crate::StateMachine;
use crate::state::StateMachineState;
use crate::state_context::StateContext;
use crate::support::lock_util::{read, write};
use crate::support::state_machine_interceptor::StateMachineInterceptor;
use crate::transition::StateMachineTransition;

/// A list of state machine interceptors that can be applied in order.
///
/// The list is shared between a machine and its executor and interceptors can
/// be registered while the machine runs, so the collection lives behind a lock
/// and is replaced copy-on-write: a notification clones one [`Arc`] instead of
/// every interceptor.
pub struct StateMachineInterceptorList<S, E> {
    interceptors: RwLock<Arc<Vec<Arc<dyn StateMachineInterceptor<S, E>>>>>,
}

impl<S, E> StateMachineInterceptorList<S, E> {
    /// Creates a new empty interceptor list.
    pub fn new() -> Self {
        Self {
            interceptors: RwLock::new(Arc::new(Vec::new())),
        }
    }

    /// Sets the interceptors, clearing any existing interceptors.
    ///
    /// # Arguments
    /// * `interceptors` - the interceptors to set
    pub fn set(&self, interceptors: Vec<Arc<dyn StateMachineInterceptor<S, E>>>) {
        *write(&self.interceptors) = Arc::new(interceptors);
    }

    /// Adds an interceptor to the list.
    ///
    /// # Arguments
    /// * `interceptor` - the interceptor to add
    pub fn add(&self, interceptor: Arc<dyn StateMachineInterceptor<S, E>>) {
        let mut guard = write(&self.interceptors);
        let mut next = Vec::with_capacity(guard.len() + 1);
        next.extend(guard.iter().cloned());
        next.push(interceptor);
        *guard = Arc::new(next);
    }

    /// Removes an interceptor from the list.
    ///
    /// # Arguments
    /// * `interceptor` - the interceptor to remove
    pub fn remove(&self, interceptor: Arc<dyn StateMachineInterceptor<S, E>>) {
        let mut guard = write(&self.interceptors);
        let next: Vec<_> = guard
            .iter()
            .filter(|item| !Arc::ptr_eq(item, &interceptor))
            .cloned()
            .collect();
        *guard = Arc::new(next);
    }

    /// Gets a snapshot of the interceptors.
    pub fn snapshot(&self) -> Arc<Vec<Arc<dyn StateMachineInterceptor<S, E>>>> {
        read(&self.interceptors).clone()
    }

    /// Pre-processes an event before it is handled by the state machine.
    ///
    /// # Arguments
    /// * `message` - the message containing the event
    /// * `state_machine` - the state machine instance
    ///
    /// # Returns
    /// The (possibly replaced) message, or `None` if processing must be vetoed
    pub fn pre_event(
        &self,
        message: Arc<dyn Message<E>>,
        state_machine: &dyn StateMachine<S, E>,
    ) -> Option<Arc<dyn Message<E>>> {
        let mut current = message;
        for interceptor in self.snapshot().iter() {
            match interceptor.pre_event(current, state_machine) {
                Ok(next) => current = next,
                Err(error) => {
                    debug!(%error, "an interceptor vetoed the event");
                    return None;
                }
            }
        }
        Some(current)
    }

    /// Pre-processes a state change before it occurs.
    ///
    /// # Arguments
    /// * `state` - the target state
    /// * `message` - the message that triggered the change
    /// * `transition` - the transition being performed
    /// * `state_machine` - the state machine instance
    /// * `root_state_machine` - the root state machine instance
    ///
    /// # Returns
    /// `false` if an interceptor stopped the state change
    pub fn pre_state_change(
        &self,
        state: &dyn StateMachineState<S, E>,
        message: Option<&dyn Message<E>>,
        transition: Option<&dyn StateMachineTransition<S, E>>,
        state_machine: &dyn StateMachine<S, E>,
        root_state_machine: &dyn StateMachine<S, E>,
    ) -> bool {
        for interceptor in self.snapshot().iter() {
            if let Err(error) = interceptor.pre_state_change(
                state,
                message,
                transition,
                state_machine,
                root_state_machine,
            ) {
                debug!(%error, "an interceptor stopped the state change");
                return false;
            }
        }
        true
    }

    /// Post-processes a state change after it has occurred.
    ///
    /// # Arguments
    /// * `state` - the new state
    /// * `message` - the message that triggered the change
    /// * `transition` - the transition that was performed
    /// * `state_machine` - the state machine instance
    /// * `root_state_machine` - the root state machine instance
    pub fn post_state_change(
        &self,
        state: &dyn StateMachineState<S, E>,
        message: Option<&dyn Message<E>>,
        transition: Option<&dyn StateMachineTransition<S, E>>,
        state_machine: &dyn StateMachine<S, E>,
        root_state_machine: &dyn StateMachine<S, E>,
    ) {
        for interceptor in self.snapshot().iter() {
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
    /// # Arguments
    /// * `state_context` - the state context
    ///
    /// # Returns
    /// `false` if an interceptor stopped the transition
    pub fn pre_transition(&self, state_context: &dyn StateContext<S, E>) -> bool {
        for interceptor in self.snapshot().iter() {
            if let Err(error) = interceptor.pre_transition(state_context) {
                debug!(%error, "an interceptor stopped the transition");
                return false;
            }
        }
        true
    }

    /// Post-processes a transition after it has occurred.
    ///
    /// The first error stops the chain and is returned to the caller, which
    /// reports it to whoever sent the event.
    ///
    /// # Arguments
    /// * `state_context` - the state context
    pub fn post_transition(
        &self,
        state_context: &dyn StateContext<S, E>,
    ) -> Result<(), BoxError> {
        for interceptor in self.snapshot().iter() {
            interceptor.post_transition(state_context)?;
        }

        Ok(())
    }

    /// Handles an error that occurred in the state machine.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine instance
    /// * `error` - the error that occurred
    ///
    /// # Returns
    /// The (possibly modified) error, or `None` if an interceptor suppressed it
    pub fn state_machine_error(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        error: BoxError,
    ) -> Option<BoxError> {
        let mut current = Some(error);
        for interceptor in self.snapshot().iter() {
            let error = current?;
            current = interceptor.state_machine_error(state_machine, error);
        }
        current
    }
}

impl<S, E> Clone for StateMachineInterceptorList<S, E> {
    fn clone(&self) -> Self {
        Self {
            interceptors: RwLock::new(self.snapshot()),
        }
    }
}

impl<S, E> Default for StateMachineInterceptorList<S, E> {
    fn default() -> Self {
        Self::new()
    }
}
