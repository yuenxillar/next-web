use std::{
    fmt::Debug,
    hash::Hash,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::error::BoxError;

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

/// The default factory, which delegates to the base factory.
#[derive(Clone)]
pub struct DefaultStateMachineFactory<S, E> {
    base: BaseStateMachineFactory<S, E>,
}

impl<S, E> DefaultStateMachineFactory<S, E>
where
    S: Eq + Hash + Debug,
    S: Clone,
    S: Send + Sync + 'static,
    E: Eq + Hash,
    E: Clone,
    E: Send + Sync + 'static,
{
    /// Creates a new factory.
    ///
    /// # Arguments
    /// * `default_state_machine_model` - the model used when no model factory is set
    /// * `state_machine_model_factory` - the factory which builds the model by machine id
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
    E: Eq + Hash,
    E: Clone,
    E: Send + Sync + 'static,
{
    /// Creates a factory from the builder configuration.
    ///
    /// # Arguments
    /// * `builder` - the builder which holds the configuration
    ///
    /// # Returns
    /// The factory, or the error which made the configuration unusable
    pub fn create(
        builder: &mut StateMachineConfigBuilder<S, E>,
    ) -> Result<Self, BoxError> {
        let state_machine_config = builder.get_or_build().ok_or_else(|| {
            crate::state_machine_exception::state_machine_error(
                crate::state_machine_exception::StateMachineErrorKind::MalformedConfiguration,
                "the state machine configuration could not be built",
            )
        })?;

        let state_machine_transitions = state_machine_config.transitions();
        let state_machine_states = state_machine_config.states();
        let state_machine_configuration_config =
            state_machine_config.state_machine_configuration_config();

        
        let state_machine_factory = if state_machine_config.model().is_some()
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

            let state_machine_model_factory = state_machine_config
                .model()
                .and_then(|model| model.factory().cloned());
            DefaultStateMachineFactory::<S, E>::new(
                default_state_machine_model,
                state_machine_model_factory,
            )
        } else {
            let default_state_machine_model = Arc::new(DefaultStateMachineModel::<S, E>::new(
                state_machine_configuration_config.clone(),
                state_machine_states.clone().into(),
                state_machine_transitions.clone().into(),
            ));
            DefaultStateMachineFactory::<S, E>::new(default_state_machine_model, None)
        };

        Ok(state_machine_factory)
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
