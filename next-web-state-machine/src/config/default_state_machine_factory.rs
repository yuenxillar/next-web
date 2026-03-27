use std::{
    fmt::Debug,
    hash::Hash,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::config::{
    base_state_machine_factory::BaseStateMachineFactory,
    builders::state_machine_config_builder::StateMachineConfigBuilder,
    common::base_configured_builder::BaseConfiguredBuilderExtOwn,
    model::{
        default_state_machine_model::DefaultStateMachineModel,
        state_machine_model::StateMachineModel,
        state_machine_model_factory::StateMachineModelFactory,
    },
};

#[derive(Clone)]
pub struct DefaultStateMachineFactory<S, E> {
    base: BaseStateMachineFactory<S, E>,
}

impl<S, E> DefaultStateMachineFactory<S, E>
where
    S: Eq + Hash + Debug,
    S: Clone,
    S: Send + Sync + 'static,
    E: Eq,
    E: Clone,
    E: Send + Sync + 'static,
{
    pub fn new(
        default_state_machine_model: Arc<dyn StateMachineModel<S, E>>,
        state_machine_model_factory: Option<Arc<dyn StateMachineModelFactory<S, E>>>,
    ) -> Self {
        Self {
            base: BaseStateMachineFactory::<S, E>::new(
                default_state_machine_model,
                state_machine_model_factory,
            ),
        }
    }
}

impl<S, E> DefaultStateMachineFactory<S, E>
where
    S: Eq + Hash + Debug,
    S: Clone,
    S: Send + Sync + 'static,
    E: Eq,
    E: Clone,
    E: Send + Sync + 'static,
{
    pub fn create(builder: &mut StateMachineConfigBuilder<S, E>) -> Self {
        let state_machine_config = builder.get_or_build().unwrap();

        let state_machine_transitions = state_machine_config.transitions();
        let state_machine_states = state_machine_config.states();
        let state_machine_configuration_config =
            state_machine_config.state_machine_configuration_config();

        let state_machine_factory;
        if state_machine_config.model().is_some()
            && state_machine_config
                .model()
                .map(|model| model.factory().is_some())
                .unwrap_or_default()
        {
            let default_state_machine_model = Arc::new(DefaultStateMachineModel::<S, E>::new(
                state_machine_configuration_config.clone(),
                None,
                None,
            ));

            let state_machine_model_factory =
                state_machine_config.model().unwrap().factory().clone();
            state_machine_factory = DefaultStateMachineFactory::<S, E>::new(
                default_state_machine_model,
                state_machine_model_factory.map(Clone::clone),
            );
        } else {
            let default_state_machine_model = Arc::new(DefaultStateMachineModel::<S, E>::new(
                state_machine_configuration_config.clone(),
                state_machine_states.clone().into(),
                state_machine_transitions.clone().into(),
            ));
            state_machine_factory =
                DefaultStateMachineFactory::<S, E>::new(default_state_machine_model, None);
        }

        state_machine_factory
    }
}

impl<S, E> Deref for DefaultStateMachineFactory<S, E> {
    type Target = BaseStateMachineFactory<S, E>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<S, E> DerefMut for DefaultStateMachineFactory<S, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
