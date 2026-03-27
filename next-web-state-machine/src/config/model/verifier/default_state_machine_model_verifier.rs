use std::marker::PhantomData;

use next_web_core::error::BoxError;

use crate::config::model::{
    state_machine_model::StateMachineModel,
    verifier::state_machine_model_verifier::StateMachineModelVerifier,
};

pub struct DefaultStateMachineModelVerifier<S, E>(PhantomData<(S, E)>);

impl<S, E> Default for DefaultStateMachineModelVerifier<S, E> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<S, E> StateMachineModelVerifier<S, E> for DefaultStateMachineModelVerifier<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    fn verify(&self, _model: &dyn StateMachineModel<S, E>) -> Result<(), BoxError> {
        Ok(())
    }
}
