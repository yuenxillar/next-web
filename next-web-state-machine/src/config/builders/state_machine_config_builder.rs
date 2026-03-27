use std::{
    hash::Hash,
    ops::{Deref, DerefMut},
};

use next_web_core::{error::BoxError, traits::required::Required};

use crate::config::{
    builders::{
        state_machine_configuration_builder::StateMachineConfigurationBuilder,
        state_machine_model_builder::StateMachineModelBuilder,
        state_machine_state_builder::StateMachineStateBuilder,
        state_machine_transition_builder::StateMachineTransitionBuilder,
    },
    common::{
        base_builder::{BaseBuilder, BaseBuilderExt},
        base_configured_builder::{
            execute_configured_build, BaseConfiguredBuilder, BaseConfiguredBuilderExt,
        },
        builder::Builder,
    },
    state_machine_config::StateMachineConfig,
};

#[derive(Clone)]
pub struct StateMachineConfigBuilder<S, E>
where
    S: Clone + 'static,
    S: Eq + Hash,
    E: Clone + 'static,
    E: Eq,
    S: Send + Sync,
    E: Send + Sync,
{
    base: BaseConfiguredBuilder<StateMachineConfig<S, E>, Self, Self>,
}

impl<S, E> StateMachineConfigBuilder<S, E>
where
    S: Clone + 'static,
    S: Eq + Hash,
    E: Clone + 'static,
    S: Send + Sync,
    E: Send + Sync,
    E: Eq,
{
}

impl<S, E> Required<BaseBuilder<StateMachineConfig<S, E>>> for StateMachineConfigBuilder<S, E>
where
    S: Clone + 'static,
    S: Eq + Hash,
    E: Clone + 'static,
    S: Send + Sync,
    E: Send + Sync,
    E: Eq,
{
    fn get_object(&self) -> &BaseBuilder<StateMachineConfig<S, E>> {
        &self.base.base
    }

    fn get_mut_object(&mut self) -> &mut BaseBuilder<StateMachineConfig<S, E>> {
        &mut self.base.base
    }
}

impl<S, E> BaseConfiguredBuilderExt<StateMachineConfig<S, E>> for StateMachineConfigBuilder<S, E>
where
    S: Clone + Eq + Hash,
    S: Send + Sync + 'static,
    E: Clone + Eq,
    E: Send + Sync + 'static,
{
    fn perform_build(&mut self) -> Result<StateMachineConfig<S, E>, BoxError> {
        let mut model_builder = self
            .get_own_shared_object::<StateMachineModelBuilder<S, E>>()
            .unwrap();
        let mut configuration_builder = self
            .get_own_shared_object::<StateMachineConfigurationBuilder<S, E>>()
            .unwrap();
        let mut transition_builder = self
            .get_own_shared_object::<StateMachineTransitionBuilder<S, E>>()
            .unwrap();
        let mut state_builder = self
            .get_own_shared_object::<StateMachineStateBuilder<S, E>>()
            .unwrap();

        let model = model_builder.build()?;

        let state_machine_configuration_config = configuration_builder.build()?;

        let transitions = transition_builder.build()?;

        let states = state_builder.build()?;

        Ok(StateMachineConfig::new(
            state_machine_configuration_config,
            transitions,
            states,
            model.into(),
        ))
    }
}

impl<S, E> BaseBuilderExt<StateMachineConfig<S, E>> for StateMachineConfigBuilder<S, E>
where
    S: Clone + Eq + Hash,
    S: Send + Sync + 'static,
    E: Clone + Eq,
    E: Send + Sync + 'static,
{
    fn do_build(&mut self) -> Result<StateMachineConfig<S, E>, BoxError> {
        execute_configured_build(self)
    }
}

impl<S, E> AsRef<Self> for StateMachineConfigBuilder<S, E>
where
    S: Clone + 'static,
    S: Eq + Hash,
    E: Clone + 'static,
    E: Eq,
    S: Send + Sync,
    E: Send + Sync,
{
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<S, E> Default for StateMachineConfigBuilder<S, E>
where
    S: Clone + 'static,
    S: Eq + Hash,
    E: Clone + 'static,
    E: Eq,

    S: Send + Sync,
    E: Send + Sync,
{
    fn default() -> Self {
        Self {
            base: Default::default(),
        }
    }
}

impl<S, E> Deref for StateMachineConfigBuilder<S, E>
where
    S: Clone + 'static,
    S: Eq + Hash,
    E: Clone + 'static,
    E: Send + Sync,
    E: Eq,

    S: Send + Sync,
    E: Send + Sync,
{
    type Target = BaseConfiguredBuilder<StateMachineConfig<S, E>, Self, Self>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<S, E> DerefMut for StateMachineConfigBuilder<S, E>
where
    S: Clone + 'static,
    S: Send + Sync,
    E: Send + Sync,
    S: Eq + Hash,
    E: Clone + 'static,
    E: Eq,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<S, E> Required<BaseConfiguredBuilder<StateMachineConfig<S, E>, Self, Self>>
    for StateMachineConfigBuilder<S, E>
where
    S: Clone + 'static,
    S: Eq + Hash,
    E: Clone + 'static,
    E: Eq,

    S: Send + Sync,
    E: Send + Sync,
{
    fn get_object(&self) -> &BaseConfiguredBuilder<StateMachineConfig<S, E>, Self, Self> {
        &self.base
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut BaseConfiguredBuilder<StateMachineConfig<S, E>, Self, Self> {
        &mut self.base
    }
}
