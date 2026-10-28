use std::sync::Arc;

use crate::{
    config::{
        builders::state_machine_configuration_configurer::StateMachineConfigurationConfigurer,
        configurer_builder::ConfigurerBuilder,
    },
    security::{access_decision_manager::AccessDecisionManager, security_rule::ComparisonType},
};

/// Base `SecurityConfigurer` interface for configuring generic config.
///
/// This trait provides methods to configure security settings for a state machine,
/// including enabling/disabling security, setting access decision managers, and
/// defining security rules for events and transitions.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
pub trait StateMachineSecurityConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineConfigurationConfigurer<S, E>>>,
{
    /// Specify if security is enabled. On default security is enabled
    /// if configurer is used.
    ///
    /// # Arguments
    /// * `enabled` - the enable flag
    ///
    /// # Returns
    /// configurer for chaining
    fn enabled(&mut self, enabled: bool) -> &mut dyn StateMachineSecurityConfigurer<S, E>;

    /// Specify a custom `AccessDecisionManager` for transitions.
    ///
    /// # Arguments
    /// * `access_decision_manager` - the access decision manager
    ///
    /// # Returns
    /// configurer for chaining
    fn transition_access_decision_manager(
        &mut self,
        access_decision_manager: Arc<dyn AccessDecisionManager>,
    ) -> &mut dyn StateMachineSecurityConfigurer<S, E>;

    /// Specify a custom `AccessDecisionManager` for events.
    ///
    /// # Arguments
    /// * `access_decision_manager` - the access decision manager
    ///
    /// # Returns
    /// configurer for chaining
    fn event_access_decision_manager(
        &mut self,
        access_decision_manager: Arc<dyn AccessDecisionManager>,
    ) -> &mut dyn StateMachineSecurityConfigurer<S, E>;

    /// Specify a security attributes for events.
    ///
    /// # Arguments
    /// * `attributes` - the security attributes
    /// * `match_type` - the match type
    ///
    /// # Returns
    /// configurer for chaining
    fn event_with_attributes(
        &mut self,
        attributes: String,
        match_type: ComparisonType,
    ) -> &mut dyn StateMachineSecurityConfigurer<S, E>;

    /// Specify a security attributes for events.
    ///
    /// # Arguments
    /// * `expression` - the security expression
    ///
    /// # Returns
    /// configurer for chaining
    fn event_with_expression(
        &mut self,
        expression: String,
    ) -> &mut dyn StateMachineSecurityConfigurer<S, E>;

    /// Specify a security attributes for transitions.
    ///
    /// # Arguments
    /// * `attributes` - the security attributes
    /// * `match_type` - the match type
    ///
    /// # Returns
    /// configurer for chaining
    fn transition_with_attributes(
        &mut self,
        attributes: String,
        match_type: ComparisonType,
    ) -> &mut dyn StateMachineSecurityConfigurer<S, E>;

    /// Specify a security attributes for transitions.
    ///
    /// # Arguments
    /// * `expression` - the security expression
    ///
    /// # Returns
    /// configurer for chaining
    fn transition_with_expression(
        &mut self,
        expression: String,
    ) -> &mut dyn StateMachineSecurityConfigurer<S, E>;
}
