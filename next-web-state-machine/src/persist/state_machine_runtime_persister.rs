use std::sync::Arc;

use crate::{
    state_machine_persist::StateMachinePersist,
    support::state_machine_interceptor::StateMachineInterceptor,
};

/// defining a runtime persistence of a StateMachine.
pub trait StateMachineRuntimePersister<S, E, T>
where
    Self: StateMachinePersist<S, E, T>,
{
    /// Gets a {StateMachineIntercepto handling machine persistence.
    fn get_interceptor(&self) -> Option<Arc<dyn StateMachineInterceptor<S, E>>>;
}
