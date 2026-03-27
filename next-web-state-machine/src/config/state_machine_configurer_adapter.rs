use std::hash::Hash;

use next_web_core::error::BoxError;

use crate::config::{
    builders::{
        state_machine_config_builder::StateMachineConfigBuilder,
        state_machine_configuration_builder::StateMachineConfigurationBuilder,
        state_machine_configurer::StateMachineConfigurer,
        state_machine_model_builder::StateMachineModelBuilder,
        state_machine_state_builder::StateMachineStateBuilder,
        state_machine_transition_builder::StateMachineTransitionBuilder,
    },
    common::configurer::Configurer,
    state_machine_config::StateMachineConfig,
};

type Result<T> = std::result::Result<T, BoxError>;

pub struct StateMachineConfigurerAdapter<T, S, E>
where
    S: Clone + Eq + Hash,
    S: Send + Sync + 'static,
    E: Clone + Eq,
    E: Send + Sync + 'static,
{
    model_builder: Option<StateMachineModelBuilder<S, E>>,
    transition_builder: Option<StateMachineTransitionBuilder<S, E>>,
    state_builder: Option<StateMachineStateBuilder<S, E>>,
    configuration_builder: Option<StateMachineConfigurationBuilder<S, E>>,

    state_machine_configurer: T,
}

impl<T, S, E> StateMachineConfigurerAdapter<T, S, E>
where
    S: Send + Sync + 'static,
    S: Eq + Hash,
    S: Clone,
    E: Send + Sync + 'static,
    E: Clone,
    E: Eq,

    T: StateMachineConfigurer<S, E>,
{
    pub fn new(
        model_builder: StateMachineModelBuilder<S, E>,
        transition_builder: StateMachineTransitionBuilder<S, E>,
        state_builder: StateMachineStateBuilder<S, E>,
        configuration_builder: StateMachineConfigurationBuilder<S, E>,
        state_machine_configurer: T,
    ) -> Self {
        Self {
            model_builder: Some(model_builder),
            transition_builder: Some(transition_builder),
            state_builder: Some(state_builder),
            configuration_builder: Some(configuration_builder),
            state_machine_configurer: state_machine_configurer,
        }
    }

    pub fn with_state_machine_configurer(state_machine_configurer: T) -> Self {
        Self {
            state_machine_configurer,
            model_builder: Default::default(),
            transition_builder: Default::default(),
            state_builder: Default::default(),
            configuration_builder: Default::default(),
        }
    }

    pub fn get_state_machine_model_builder(&mut self) -> Result<StateMachineModelBuilder<S, E>> {
        if let Some(model_builder) = self.model_builder.take() {
            return Ok(model_builder);
        }

        let mut model_builder = StateMachineModelBuilder::new(true);
        self.state_machine_configurer
            .model_configure(&mut model_builder)?;

        Ok(model_builder)
    }

    pub fn get_state_machine_transition_builder(
        &mut self,
    ) -> Result<StateMachineTransitionBuilder<S, E>> {
        if let Some(transition_builder) = self.transition_builder.take() {
            return Ok(transition_builder);
        }

        let mut transition_builder = StateMachineTransitionBuilder::new(true);
        self.state_machine_configurer
            .transition_configure(&mut transition_builder)?;

        Ok(transition_builder)
    }

    pub fn get_state_machine_state_builder(&mut self) -> Result<StateMachineStateBuilder<S, E>> {
        if let Some(state_builder) = self.state_builder.take() {
            return Ok(state_builder);
        }

        let mut state_builder = StateMachineStateBuilder::new(true);
        self.state_machine_configurer
            .state_configure(&mut state_builder)?;

        Ok(state_builder)
    }

    pub fn get_state_machine_configuration_builder(
        &mut self,
    ) -> Result<StateMachineConfigurationBuilder<S, E>> {
        if let Some(configuration_builder) = self.configuration_builder.take() {
            return Ok(configuration_builder);
        }

        let mut configuration_builder = StateMachineConfigurationBuilder::new(true);
        self.state_machine_configurer
            .config_configure(&mut configuration_builder)?;

        Ok(configuration_builder)
    }

    pub fn set_state_machine_configurer(&mut self, state_machine_configurer: T) {
        self.state_machine_configurer = state_machine_configurer;
    }
}

impl<T, S, E> Configurer<StateMachineConfig<S, E>, StateMachineConfigBuilder<S, E>>
    for StateMachineConfigurerAdapter<T, S, E>
where
    S: Send + Sync + 'static,
    S: Clone,
    S: Eq + Hash,
    E: Send + Sync + 'static,
    E: Eq,
    E: Clone,

    T: StateMachineConfigurer<S, E>,
    T: Clone,
{
    fn init(&mut self, config: &StateMachineConfigBuilder<S, E>) -> Result<()> {
        config.set_shared_object(self.get_state_machine_model_builder()?);
        config.set_shared_object(self.get_state_machine_transition_builder()?);
        config.set_shared_object(self.get_state_machine_state_builder()?);
        config.set_shared_object(self.get_state_machine_configuration_builder()?);

        Ok(())
    }

    fn configure(&mut self, builder: &StateMachineConfigBuilder<S, E>) -> Result<()> {
        self.state_machine_configurer.configure(builder)
    }

    fn is_assignable(&self, builder: &StateMachineConfigBuilder<S, E>) -> bool {
        self.state_machine_configurer.is_assignable(builder)
    }
}

impl<T, S, E> Clone for StateMachineConfigurerAdapter<T, S, E>
where
    S: Eq + Hash,
    S: Send + Sync + 'static,
    S: Clone,
    E: Send + Sync + 'static,
    E: Clone,
    E: Eq,

    T: StateMachineConfigurer<S, E>,
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            model_builder: self.model_builder.clone(),
            transition_builder: self.transition_builder.clone(),
            state_builder: self.state_builder.clone(),
            configuration_builder: self.configuration_builder.clone(),
            state_machine_configurer: self.state_machine_configurer.clone(),
        }
    }
}
