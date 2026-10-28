use std::sync::Arc;

use crate::{
    config::{
        builders::state_machine_configuration_configurer::StateMachineConfigurationConfigurer,
        configurer_builder::ConfigurerBuilder,
    },
    monitor::state_machine_monitor::StateMachineMonitor,
};

/// Base MonitoringConfigurer interface for configuring state machine monitoring.
pub trait MonitoringConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineConfigurationConfigurer<S, E>>>,
{
    /// Specify a state machine monitor.
    fn monitor(
        &mut self,
        monitor: Arc<dyn StateMachineMonitor<S, E>>,
    ) -> &mut dyn MonitoringConfigurer<S, E>;
}
