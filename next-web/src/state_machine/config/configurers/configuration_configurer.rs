use std::{sync::Arc, time::Duration};

use crate::state_machine::{
    action::state_do_action_policy::StateDoActionPolicy,
    config::{
        builders::state_machine_configuration_configurer::StateMachineConfigurationConfigurer,
        configurer_builder::ConfigurerBuilder,
    },
    listener::state_machine_listener::StateMachineListener,
    region::region_execution_policy::RegionExecutionPolicy,
    transition::transition_conflict_policy::TransitionConflictPolicy,
};

/// Base `ConfigurationConfigurer` interface for configuring generic config.
///
/// This trait provides methods to configure various aspects of a state machine
/// such as machine identification, bean factory integration, startup behavior,
/// listeners, conflict resolution policies, and execution policies.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
pub trait ConfigurationConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineConfigurationConfigurer<S, E>>>,
{
    /// Specify a machine identifier.
    ///
    /// # Arguments
    /// * `id` - the machine identifier
    ///
    /// # Returns
    /// configurer for chaining
    fn machine_id(&mut self, id: String) -> &mut dyn ConfigurationConfigurer<S, E>;

    /// Specify if state machine should be started automatically.
    /// On default state machine is not started automatically.
    ///
    /// # Arguments
    /// * `auto_startup` - the autoStartup flag
    ///
    /// # Returns
    /// configurer for chaining
    fn auto_startup(&mut self, auto_startup: bool) -> &mut dyn ConfigurationConfigurer<S, E>;

    /// Specify a `StateMachineListener` to be registered
    /// with a state machine. This method can be called multiple times
    /// to register multiple listeners.
    ///
    /// # Arguments
    /// * `listener` - the listener to register
    ///
    /// # Returns
    /// the configuration configurer
    fn listener(
        &mut self,
        listener: Arc<dyn StateMachineListener<S, E>>,
    ) -> &mut dyn ConfigurationConfigurer<S, E>;

    /// Specify a `TransitionConflictPolicy`. Default to `TransitionConflictPolicy::CHILD`.
    ///
    /// # Arguments
    /// * `transition_conflict_policy` - the transition conflict policy
    ///
    /// # Returns
    /// the configuration configurer
    fn transition_conflict_policy(
        &mut self,
        transition_conflict_policy: TransitionConflictPolicy,
    ) -> &mut dyn ConfigurationConfigurer<S, E>;

    /// Specify a `StateDoActionPolicy`. Defaults to `StateDoActionPolicy::IMMEDIATE_CANCEL`.
    ///
    /// # Arguments
    /// * `state_do_action_policy` - the state do action policy
    ///
    /// # Returns
    /// the configuration configurer
    fn state_do_action_policy(
        &mut self,
        state_do_action_policy: StateDoActionPolicy,
    ) -> &mut dyn ConfigurationConfigurer<S, E>;

    /// Specify a timeout used with `StateDoActionPolicy`.
    ///
    /// # Arguments
    /// * `timeout` - the timeout duration
    ///
    /// # Returns
    /// the configuration configurer
    fn state_do_action_policy_timeout(
        &mut self,
        timeout: Duration,
    ) -> &mut dyn ConfigurationConfigurer<S, E>;

    /// Specify a `RegionExecutionPolicy`. Default to `RegionExecutionPolicy::SEQUENTIAL`.
    ///
    /// # Arguments
    /// * `region_execution_policy` - the region execution policy
    ///
    /// # Returns
    /// the configuration configurer
    fn region_execution_policy(
        &mut self,
        region_execution_policy: RegionExecutionPolicy,
    ) -> &mut dyn ConfigurationConfigurer<S, E>;
}
