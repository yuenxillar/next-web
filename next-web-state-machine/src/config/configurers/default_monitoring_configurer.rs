use std::sync::Arc;

use next_web_core::error::BoxError;

use crate::{
    config::{
        builders::{
            state_machine_configuration_builder::StateMachineConfigurationBuilder,
            state_machine_configuration_configurer::StateMachineConfigurationConfigurer,
        },
        common::configurer_adapter::{ConfigurerAdapter, ConfigurerAdapterExt},
        configurer_builder::ConfigurerBuilder,
        configurers::monitoring_configurer::MonitoringConfigurer,
        model::configuration_data::ConfigurationData,
    },
    monitor::state_machine_monitor::StateMachineMonitor,
};

/// Default implementation of a `MonitoringConfigurer`.
pub struct DefaultMonitoringConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    monitor: Option<Arc<dyn StateMachineMonitor<S, E>>>,

    pub(crate) base: ConfigurerAdapter<
        ConfigurationData<S, E>,
        Box<dyn StateMachineConfigurationConfigurer<S, E>>,
        StateMachineConfigurationBuilder<S, E>,
    >,
}

impl<S, E> DefaultMonitoringConfigurer<S, E>
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
    for DefaultMonitoringConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn and(&mut self) -> Box<dyn StateMachineConfigurationConfigurer<S, E>> {
        todo!()
    }
}

impl<S, E> MonitoringConfigurer<S, E> for DefaultMonitoringConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn monitor(
        &mut self,
        monitor: Arc<dyn StateMachineMonitor<S, E>>,
    ) -> &mut dyn MonitoringConfigurer<S, E> {
        self.monitor = Some(monitor);

        self
    }
}

impl<S, E> ConfigurerAdapterExt<ConfigurationData<S, E>, StateMachineConfigurationBuilder<S, E>>
    for DefaultMonitoringConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn configure(
        &mut self,
        builder: &mut StateMachineConfigurationBuilder<S, E>,
    ) -> Result<(), BoxError> {
        self.monitor
            .as_ref()
            .map(Clone::clone)
            .map(|monitor| builder.set_state_machine_monitor(monitor));

        Ok(())
    }
}

impl<S, E> Default for DefaultMonitoringConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn default() -> Self {
        Self {
            monitor: None,

            base: Default::default(),
        }
    }
}
