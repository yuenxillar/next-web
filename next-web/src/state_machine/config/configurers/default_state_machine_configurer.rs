use std::sync::Arc;
use std::time::Duration;

use next_web_core::error::BoxError;

use crate::state_machine::action::state_do_action_policy::StateDoActionPolicy;
use crate::state_machine::config::builders::state_machine_configuration_builder::StateMachineConfigurationBuilder;
use crate::state_machine::config::builders::state_machine_configuration_configurer::StateMachineConfigurationConfigurer;
use crate::state_machine::config::common::configurer_adapter::{
    ConfigurerAdapter, ConfigurerAdapterExt,
};
use crate::state_machine::config::configurer_builder::ConfigurerBuilder;
use crate::state_machine::config::configurers::configuration_configurer::ConfigurationConfigurer;
use crate::state_machine::config::model::configuration_data::ConfigurationData;
use crate::state_machine::listener::state_machine_listener::StateMachineListener;
use crate::state_machine::region::region_execution_policy::RegionExecutionPolicy;
use crate::state_machine::transition::transition_conflict_policy::TransitionConflictPolicy;

/// Configuration data for state machine.
#[derive(Clone)]
pub struct DefaultConfigurationConfigurer<S, E> {
    /// Machine identifier.
    machine_id: Option<String>,
    /// Whether to auto-start the machine.
    auto_start: bool,
    /// Transition conflict policy.
    transition_conflict_policy: Option<TransitionConflictPolicy>,
    /// State do-action policy.
    state_do_action_policy: Option<StateDoActionPolicy>,
    /// State do-action policy timeout.
    state_do_action_policy_timeout: Option<Duration>,
    /// Region execution policy.
    region_execution_policy: Option<RegionExecutionPolicy>,
    /// State machine listeners.
    listeners: Vec<Arc<dyn StateMachineListener<S, E>>>,

    pub(crate) base: ConfigurerAdapter<
        ConfigurationData<S, E>,
        Box<dyn StateMachineConfigurationConfigurer<S, E>>,
        StateMachineConfigurationBuilder<S, E>,
    >,
}

impl<S, E> ConfigurerBuilder<Box<dyn StateMachineConfigurationConfigurer<S, E>>>
    for DefaultConfigurationConfigurer<S, E>
{
    fn and(self) -> Box<dyn StateMachineConfigurationConfigurer<S, E>> {
        todo!()
    }
}

impl<S, E> ConfigurationConfigurer<S, E> for DefaultConfigurationConfigurer<S, E> {
    fn machine_id(&mut self, id: String) -> &mut dyn ConfigurationConfigurer<S, E> {
        self.machine_id = Some(id);
        self
    }

    fn auto_startup(&mut self, auto_startup: bool) -> &mut dyn ConfigurationConfigurer<S, E> {
        self.auto_start = auto_startup;
        self
    }

    fn listener(
        &mut self,
        listener: Arc<dyn StateMachineListener<S, E>>,
    ) -> &mut dyn ConfigurationConfigurer<S, E> {
        self.listeners.push(listener);
        self
    }

    fn transition_conflict_policy(
        &mut self,
        transition_conflict_policy: TransitionConflictPolicy,
    ) -> &mut dyn ConfigurationConfigurer<S, E> {
        self.transition_conflict_policy = Some(transition_conflict_policy);
        self
    }

    fn state_do_action_policy(
        &mut self,
        state_do_action_policy: StateDoActionPolicy,
    ) -> &mut dyn ConfigurationConfigurer<S, E> {
        self.state_do_action_policy = Some(state_do_action_policy);
        self
    }

    fn state_do_action_policy_timeout(
        &mut self,
        timeout: Duration,
    ) -> &mut dyn ConfigurationConfigurer<S, E> {
        self.state_do_action_policy_timeout = Some(timeout);
        self
    }

    fn region_execution_policy(
        &mut self,
        region_execution_policy: RegionExecutionPolicy,
    ) -> &mut dyn ConfigurationConfigurer<S, E> {
        self.region_execution_policy = Some(region_execution_policy);
        self
    }
}

impl<S, E> ConfigurerAdapterExt<ConfigurationData<S, E>, StateMachineConfigurationBuilder<S, E>>
    for DefaultConfigurationConfigurer<S, E>
{
    fn configure(
        &mut self,
        builder: &mut StateMachineConfigurationBuilder<S, E>,
    ) -> Result<(), BoxError> {
        self.machine_id.as_ref().map(|id| {
            builder.set_machine_id(id.clone());
        });

        builder
            .set_auto_start(self.auto_start)
            .set_state_machine_listeners(self.listeners.clone());

        self.transition_conflict_policy.as_ref().map(|policy| {
            builder.set_transition_conflict_policy(policy.clone());
        });

        self.state_do_action_policy.as_ref().map(|policy| {
            builder.set_state_do_action_policy(
                policy.clone(),
                self.state_do_action_policy_timeout
                    .unwrap_or(Duration::from_millis(1)),
            );
        });

        self.region_execution_policy.as_ref().map(|policy| {
            builder.set_region_execution_policy(policy.clone());
        });

        Ok(())
    }
}

impl<S, E> Default for DefaultConfigurationConfigurer<S, E> {
    fn default() -> Self {
        Self {
            machine_id: None,
            auto_start: false,
            transition_conflict_policy: None,
            state_do_action_policy: None,
            state_do_action_policy_timeout: None,
            region_execution_policy: None,
            listeners: Vec::new(),

            base: Default::default(),
        }
    }
}
