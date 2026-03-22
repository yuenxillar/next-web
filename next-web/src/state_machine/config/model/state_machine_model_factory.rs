use std::sync::Arc;

use crate::state_machine::config::model::state_machine_model::StateMachineModel;

/// A generic builder interface for building StateMachineModels.
pub trait StateMachineModelFactory<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    /// Builds the state machine model.
    fn build(&self) -> Arc<dyn StateMachineModel<S, E>>;

    /// Builds the state machine model with a given machineId. Implementation is free to choose what to
    /// do with a given machineId but usually it might map to a different configurations supported by
    /// storage or repository in a factory.
    fn build_with_machine_id(&self, machine_id: String) -> Arc<dyn StateMachineModel<S, E>>;
}
