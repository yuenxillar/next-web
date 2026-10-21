use std::sync::Arc;

use next_web_core::error::BoxError;

use crate::state_machine_context::StateMachineContext;

/// StateMachinePersist is an interface handling serialization
pub trait StateMachinePersist<S, E, T>
where
    Self: Send + Sync,
{
    /// Write a StateMachineContext into a persistent store
    fn write(
        &self,
        context: &dyn StateMachineContext<S, E>,
        context_obj: T,
    ) -> Result<(), BoxError>;

    /// Read a StateMachineContext from a persistent store
    ///
    /// A store which has nothing for the key has to report
    /// [`StateMachineErrorKind::MachineNotFound`](crate::state_machine_exception::StateMachineErrorKind::MachineNotFound),
    /// because that is how a service tells "nothing was stored yet" from "the
    /// store failed" and builds a fresh machine instead of returning an error.
    fn read(&self, context_obj: T) -> Result<Arc<dyn StateMachineContext<S, E>>, BoxError>;
}
