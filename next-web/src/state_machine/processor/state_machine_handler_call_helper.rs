use std::marker::PhantomData;

use next_web_core::error::BoxError;

#[derive(Clone)]
pub struct StateMachineHandlerCallHelper<S, E> {
    var: PhantomData<(S, E)>,
}

impl<S, E> StateMachineHandlerCallHelper<S, E> {
    pub async fn after_properties_set(&self) -> Result<(), BoxError> {
        Ok(())
    }
}

impl<S, E> Default for StateMachineHandlerCallHelper<S, E> {
    fn default() -> Self {
        Self { var: PhantomData }
    }
}
