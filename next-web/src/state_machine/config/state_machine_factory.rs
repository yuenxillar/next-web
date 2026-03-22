use std::sync::Arc;

use next_web_core::error::BoxError;
use uuid::Uuid;

use crate::state_machine::StateMachine;

/// StateMachineFactory is a strategy interface building StateMachines.
pub trait StateMachineFactory<S, E> {
    /// Build a new StateMachine instance.
    fn get_state_machine(&self) -> Result<Arc<dyn StateMachine<S, E>>, BoxError>;

    /// Build a new StateMachine instance with a given machine id.
    fn get_state_machine_with_id(
        &self,
        machine_id: String,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError>;

    fn get_state_machine_with_uuid(
        &self,
        machine_id: Uuid,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError>;

    // fn create(builder: StateMachineConfigBuilder<S, E>) {}
}
