use std::any::Any;
use std::hash::Hash;
use std::sync::Arc;

use next_web_core::BoxFuture;
use next_web_core::error::BoxError;

use crate::access::state_machine_accessor::StateMachineAccessor;
use crate::extended_state::ExtendedState;
use crate::region::Region;
use crate::state::StateMachineState;
use crate::state::pseudo_state::PseudoState;
use crate::state_context::StateContext;
use crate::support::state_machine_interceptor::StateMachineInterceptor;

/// A boxed state action, shared between states and transitions.
pub type BoxedStateAction<S, E> =
    Arc<dyn for<'a> Fn(&'a dyn StateContext<S, E>) -> BoxFuture<'a, Result<(), BoxError>> + Send + Sync>;

/// A boxed state guard, shared between states and transitions.
pub type BoxedStateGuard<S, E> =
    Arc<dyn for<'a> Fn(&'a dyn StateContext<S, E>) -> BoxFuture<'a, bool> + Send + Sync>;

/// `StateMachine` is a model of a state machine, and the runtime that executes
/// it.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub trait StateMachine<S, E>: Region<S, E> + Any + Send + Sync
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Gets the initial state of the machine.
    fn initial_state(&self) -> &dyn StateMachineState<S, E>;

    /// Gets the extended state of the machine.
    fn extended_state(&self) -> &dyn ExtendedState;

    /// Gets the accessor which exposes the regions of the machine.
    fn state_machine_accessor(&self) -> &dyn StateMachineAccessor<S, E>;

    /// Puts the machine into an error state.
    ///
    /// # Arguments
    /// * `error` - the error which caused the state
    fn set_state_machine_error(&self, error: BoxError);

    /// Checks whether the machine is in an error state.
    fn has_state_machine_error(&self) -> bool;

    /// Checks whether the machine is running.
    fn is_running(&self) -> bool;

    /// Adds a state machine interceptor.
    ///
    /// # Arguments
    /// * `interceptor` - the interceptor to add
    fn add_state_machine_interceptor(
        &self,
        interceptor: Arc<dyn StateMachineInterceptor<S, E>>,
    ) -> Result<(), BoxError>;

    /// Adds a state machine monitor.
    ///
    /// # Arguments
    /// * `monitor` - the monitor to add
    fn add_state_machine_monitor(
        &self,
        monitor: Arc<dyn crate::monitor::state_machine_monitor::StateMachineMonitor<S, E>>,
    ) -> Result<(), BoxError>;

    /// Reports the duration of an executed action to the monitors.
    ///
    /// # Arguments
    /// * `action` - the action which ran
    /// * `duration` - how long the action took
    fn notify_action_executed(&self, action: &BoxedStateAction<S, E>, duration: std::time::Duration) {
        let _ = (action, duration);
    }

    /// Gets the history pseudo state of the machine, if any.
    fn get_history_state(&self) -> Option<Arc<dyn PseudoState<S, E>>>;

    /// Captures the current state of the machine so it can be persisted.
    ///
    /// # Returns
    /// The context which describes the machine, or an error when the machine has
    /// no current state yet
    fn snapshot_context(
        &self,
    ) -> Result<Arc<dyn crate::state_machine_context::StateMachineContext<S, E>>, BoxError>;

    /// Restores the machine to the given state without running entry or exit
    /// actions.
    ///
    /// # Arguments
    /// * `state` - the identifier of the state to restore to
    fn reset_to(&self, state: &S) -> Result<(), BoxError>;

    /// Restores the state and the extended state variables of a context.
    ///
    /// The extended state variables which the context carries replace the ones
    /// of a machine which is restored from a store. The initial transition is
    /// disabled, so starting the restored machine continues in the restored
    /// state instead of re-entering the initial one.
    ///
    /// # Arguments
    /// * `context` - the context to restore
    fn restore_context(
        &self,
        context: &dyn crate::state_machine_context::StateMachineContext<S, E>,
    ) -> Result<(), BoxError> {
        self.reset_to(context.state())?;

        for (key, value) in context.extended_state().get_variables() {
            self.extended_state().set(key, value);
        }

        self.set_initial_enabled(false);

        Ok(())
    }
}

/// A trait bound alias for the identifier type of a state machine.
pub trait StateMachineId: Clone + Eq + Hash + Send + Sync + 'static {}

impl<T> StateMachineId for T where T: Clone + Eq + Hash + Send + Sync + 'static {}
