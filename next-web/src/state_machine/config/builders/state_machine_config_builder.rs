use next_web_core::error::BoxError;

use crate::state_machine::config::{
    common::builder::Builder, state_machine_config::StateMachineConfig,
};

#[derive(Default)]
pub struct StateMachineConfigBuilder<S, E> {
    pub var: std::marker::PhantomData<(S, E)>,
}

impl<S, E> StateMachineConfigBuilder<S, E> {}

impl<S, E> Builder<StateMachineConfig<S, E>> for StateMachineConfigBuilder<S, E> {
    fn build(&mut self) -> Result<StateMachineConfig<S, E>, BoxError> {
        Ok(StateMachineConfig::default())
    }
}
