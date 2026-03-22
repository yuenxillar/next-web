use std::hash::Hash;

use next_web_core::error::BoxError;

use crate::state_machine::config::{
    builders::{
        state_machine_config_builder::StateMachineConfigBuilder,
        state_machine_configuration_builder::StateMachineConfigurationBuilder,
        state_machine_configuration_configurer::StateMachineConfigurationConfigurer,
        state_machine_configurer::StateMachineConfigurer,
        state_machine_model_builder::StateMachineModelBuilder,
        state_machine_model_configurer::StateMachineModelConfigurer,
        state_machine_state_builder::StateMachineStateBuilder,
        state_machine_state_configurer::StateMachineStateConfigurer,
        state_machine_transition_builder::StateMachineTransitionBuilder,
    },
    common::configurer::Configurer,
    state_machine_config::StateMachineConfig,
};

pub struct StateMachineConfigurerAdapter<'a, S, E>
where
    S: Eq + Hash,
{
    model_builder: Option<StateMachineModelBuilder<S, E>>,
    transition_builder: Option<StateMachineTransitionBuilder<S, E>>,
    state_builder: Option<StateMachineStateBuilder<S, E>>,
    configuration_builder: Option<StateMachineConfigurationBuilder<S, E>>,

    state_machine_configurer: Option<&'a mut dyn StateMachineConfigurer<S, E>>,
}

impl<'a, S, E> StateMachineConfigurerAdapter<'a, S, E>
where
    S: Send + Sync + 'static,
    S: Eq + Hash,
    S: Clone,
    E: Send + Sync + 'static,
    E: Clone,
{
    pub fn new(
        model_builder: StateMachineModelBuilder<S, E>,
        transition_builder: StateMachineTransitionBuilder<S, E>,
        state_builder: StateMachineStateBuilder<S, E>,
        configuration_builder: StateMachineConfigurationBuilder<S, E>,
        state_machine_configurer: &'a mut dyn StateMachineConfigurer<S, E>,
    ) -> Self {
        Self {
            model_builder: Some(model_builder),
            transition_builder: Some(transition_builder),
            state_builder: Some(state_builder),
            configuration_builder: Some(configuration_builder),
            state_machine_configurer: Some(state_machine_configurer),
        }
    }

    pub fn get_state_machine_model_builder(&mut self) -> StateMachineModelBuilder<S, E> {
        if let Some(model_builder) = self.model_builder.take() {
            return model_builder;
        }

        let mut model_builder = StateMachineModelBuilder::new(true);
        self.state_machine_configurer
            .as_mut()
            .unwrap()
            .model_configure(&mut model_builder);

        model_builder
    }

    pub fn get_state_machine_transition_builder(&mut self) -> StateMachineTransitionBuilder<S, E> {
        if let Some(transition_builder) = self.transition_builder.take() {
            return transition_builder;
        }

        let mut transition_builder = StateMachineTransitionBuilder::new(true);
        self.state_machine_configurer
            .as_mut()
            .unwrap()
            .transition_configure(&mut transition_builder);

        transition_builder
    }

    pub fn get_state_machine_state_builder(&mut self) -> StateMachineStateBuilder<S, E> {
        if let Some(state_builder) = self.state_builder.take() {
            return state_builder;
        }

        let mut state_builder = StateMachineStateBuilder::new(true);
        self.state_machine_configurer
            .as_mut()
            .unwrap()
            .state_configure(&mut state_builder);

        state_builder
    }

    pub fn get_state_machine_configuration_builder(
        &mut self,
    ) -> StateMachineConfigurationBuilder<S, E> {
        if let Some(configuration_builder) = self.configuration_builder.take() {
            return configuration_builder;
        }

        let mut configuration_builder = StateMachineConfigurationBuilder::new(true);
        self.state_machine_configurer
            .as_mut()
            .unwrap()
            .config_configure(&mut configuration_builder);

        configuration_builder
    }
}

impl<'a, S, E> Configurer<StateMachineConfig<S, E>, StateMachineConfigBuilder<S, E>>
    for StateMachineConfigurerAdapter<'a, S, E>
where
    S: Send + Sync,
    S: Clone,
    S: Eq + Hash,
    E: Send + Sync,
    E: Clone,
{
    fn init(&mut self, builder: &StateMachineConfigBuilder<S, E>) -> Result<(), BoxError> {
        Ok(())
    }

    fn configure(&mut self, builder: &StateMachineConfigBuilder<S, E>) -> Result<(), BoxError> {
        if let Some(configurer) = self.state_machine_configurer.as_mut() {
            return configurer.configure(builder);
        }

        Ok(())
    }

    fn is_assignable(&self, builder: &StateMachineConfigBuilder<S, E>) -> bool {
        self.state_machine_configurer
            .as_ref()
            .map(|configurer| configurer.is_assignable(builder))
            .unwrap_or_default()
    }
}

impl<'a, S, E> Clone for StateMachineConfigurerAdapter<'a, S, E>
where
    S: Eq + Hash,
    S: Clone,
    E: Clone,
{
    fn clone(&self) -> Self {
        Self {
            model_builder: self.model_builder.clone(),
            transition_builder: self.transition_builder.clone(),
            state_builder: self.state_builder.clone(),
            configuration_builder: self.configuration_builder.clone(),
            state_machine_configurer: None,
        }
    }
}

use std::marker::PhantomData;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TestState {
    Ready,
    Run,
    Run2,
    End,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TestEvent {
    Open,
    Close,
}
#[derive(Clone)]
struct DefaultStateMachineConfigurerAdapter<S, E> {
    var: PhantomData<(S, E)>,
}

impl StateMachineConfigurer<TestState, TestEvent>
    for DefaultStateMachineConfigurerAdapter<TestState, TestEvent>
{
    fn config_configure(
        &mut self,
        config: &mut StateMachineConfigurationBuilder<TestState, TestEvent>,
    ) -> Result<(), BoxError> {
        config.with_verifier()?.verifier(todo!()).enabled(true);
        Ok(())
    }

    fn model_configure(
        &mut self,
        model: &mut StateMachineModelBuilder<TestState, TestEvent>,
    ) -> Result<(), BoxError> {
        model.with_model()?.factory(todo!());
        Ok(())
    }

    fn state_configure(
        &mut self,
        states: &mut StateMachineStateBuilder<TestState, TestEvent>,
    ) -> Result<(), BoxError> {
        states
            .with_states()?
            .initial(TestState::Ready)
            .end(TestState::End)
            .states(vec![TestState::Ready, TestState::Run, TestState::End]);

        Ok(())
    }

    fn transition_configure(
        &mut self,
        transitions: &mut StateMachineTransitionBuilder<TestState, TestEvent>,
    ) -> Result<(), BoxError> {
        // transitions
        //     .with_external()?
        //     .source(TestState::Ready)
        //     .target(TestState::Run)
        //     .event(TestEvent::Open);

        Ok(())
    }
}

impl
    Configurer<
        StateMachineConfig<TestState, TestEvent>,
        StateMachineConfigBuilder<TestState, TestEvent>,
    > for DefaultStateMachineConfigurerAdapter<TestState, TestEvent>
{
    fn init(
        &mut self,
        builder: &StateMachineConfigBuilder<TestState, TestEvent>,
    ) -> Result<(), BoxError> {
        Ok(())
    }

    fn configure(
        &mut self,
        builder: &StateMachineConfigBuilder<TestState, TestEvent>,
    ) -> Result<(), BoxError> {
        Ok(())
    }

    fn is_assignable(&self, builder: &StateMachineConfigBuilder<TestState, TestEvent>) -> bool {
        false
    }
}
