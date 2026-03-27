use std::{fmt::Debug, sync::Arc};

use next_web_core::error::BoxError;

use crate::config::{
    builders::{
        state_machine_configuration_builder::StateMachineConfigurationBuilder,
        state_machine_configuration_configurer::StateMachineConfigurationConfigurer,
    },
    common::configurer_adapter::{ConfigurerAdapter, ConfigurerAdapterExt},
    configurer_builder::ConfigurerBuilder,
    configurers::verifier_configurer::StateMachineModelVerifierConfigurer,
    model::{
        configuration_data::ConfigurationData,
        verifier::{
            composite_state_machine_model_verifier::CompositeStateMachineModelVerifier,
            state_machine_model_verifier::StateMachineModelVerifier,
        },
    },
};

/// Default implementation of a `VerifierConfigurer`.
pub struct DefaultVerifierConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    enabled: bool,
    verifier: Option<Arc<dyn StateMachineModelVerifier<S, E>>>,

    pub(crate) base: ConfigurerAdapter<
        ConfigurationData<S, E>,
        Box<dyn StateMachineConfigurationConfigurer<S, E>>,
        StateMachineConfigurationBuilder<S, E>,
    >,
}

impl<S, E> DefaultVerifierConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    /// Creates a new verifier configurer.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S, E> ConfigurerBuilder<Box<dyn StateMachineConfigurationConfigurer<S, E>>>
    for DefaultVerifierConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn and(&mut self) -> Box<dyn StateMachineConfigurationConfigurer<S, E>> {
        todo!()
    }
}

impl<S, E> StateMachineModelVerifierConfigurer<S, E> for DefaultVerifierConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn enabled(&mut self, enabled: bool) -> &mut dyn StateMachineModelVerifierConfigurer<S, E> {
        self.enabled = enabled;
        self
    }

    fn verifier(
        &mut self,
        verifier: Arc<dyn StateMachineModelVerifier<S, E>>,
    ) -> &mut dyn StateMachineModelVerifierConfigurer<S, E> {
        self.verifier = Some(verifier);
        self
    }
}

impl<S, E> ConfigurerAdapterExt<ConfigurationData<S, E>, StateMachineConfigurationBuilder<S, E>>
    for DefaultVerifierConfigurer<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
    S: Debug + PartialEq + Clone,
    E: Clone,
{
    fn configure(
        &mut self,
        builder: &mut StateMachineConfigurationBuilder<S, E>,
    ) -> Result<(), BoxError> {
        builder.set_verifier_enabled(self.enabled);

        if let Some(verifier) = &self.verifier {
            builder.set_verifier(verifier.clone());
        } else {
            // Create a default composite verifier
            builder.set_verifier(Arc::new(CompositeStateMachineModelVerifier::default()));
        }

        Ok(())
    }
}

impl<S, E> Default for DefaultVerifierConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn default() -> Self {
        Self {
            enabled: true,
            verifier: None,

            base: Default::default(),
        }
    }
}
