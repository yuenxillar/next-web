use next_web_core::error::BoxError;

use crate::config::model::state_machine_model::StateMachineModel;

/// Strategy interface for implementations verifying StateMachineModel structures.
pub trait StateMachineModelVerifier<S, E>
where
    Self: Send + Sync,
{
    /// Verify a state machine model.
    fn verify(&self, model: &dyn StateMachineModel<S, E>) -> Result<(), BoxError>;
}
