use next_web_core::BoxFuture;

use crate::state_machine_context::StateMachineContext;

/// Functional interface exposing reactive `StateMachine` internals.
pub trait ReactiveStateMachineAccess<S, E> {
    /// Reset state machine reactively.
    ///
    /// # Arguments
    /// * `state_machine_context` - The state machine context
    ///
    /// # Returns
    /// A future for completion
    fn reset_state_machine_reactively(
        &self,
        state_machine_context: &dyn StateMachineContext<S, E>,
    ) -> BoxFuture<'static, ()>;
}
