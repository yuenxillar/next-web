use std::sync::Arc;

use next_web_core::error::BoxError;

use crate::state_machine_context::StateMachineContext;

/// StateMachinePersist is an interface handling serialization
pub trait StateMachinePersist<S, E, T> {
    /// Write a StateMachineContext into a persistent store
    fn write(
        &self,
        context: &dyn StateMachineContext<S, E>,
        context_obj: T,
    ) -> Result<(), BoxError>;

    /// Read a  StateMachineContext from a persistent store
    fn read(&self, context_obj: T) -> Result<Arc<dyn StateMachineContext<S, E>>, BoxError>;
}
