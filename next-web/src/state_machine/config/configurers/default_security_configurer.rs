use std::sync::Arc;

use next_web_core::error::BoxError;

use crate::state_machine::{
    config::{
        builders::{
            state_machine_configuration_builder::StateMachineConfigurationBuilder,
            state_machine_configuration_configurer::StateMachineConfigurationConfigurer,
        },
        common::configurer_adapter::{ConfigurerAdapter, ConfigurerAdapterExt},
        configurer_builder::ConfigurerBuilder,
        configurers::security_configurer::StateMachineSecurityConfigurer,
        model::configuration_data::ConfigurationData,
    },
    security::{
        access_decision_manager::AccessDecisionManager,
        security_rule::{ComparisonType, SecurityRule},
    },
};

/// Default implementation of a `SecurityConfigurer`.
pub struct DefaultSecurityConfigurer<S, E> {
    enabled: bool,
    transition_access_decision_manager: Option<Arc<dyn AccessDecisionManager>>,
    event_access_decision_manager: Option<Arc<dyn AccessDecisionManager>>,
    event_security_rule: Option<SecurityRule>,
    transition_security_rule: Option<SecurityRule>,

    pub(crate) base: ConfigurerAdapter<
        ConfigurationData<S, E>,
        Box<dyn StateMachineConfigurationConfigurer<S, E>>,
        StateMachineConfigurationBuilder<S, E>,
    >,
}

impl<S, E> DefaultSecurityConfigurer<S, E> {
    /// Creates a new security configurer.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S, E> ConfigurerBuilder<Box<dyn StateMachineConfigurationConfigurer<S, E>>>
    for DefaultSecurityConfigurer<S, E>
{
    fn and(self) -> Box<dyn StateMachineConfigurationConfigurer<S, E>> {
        todo!()
    }
}

impl<S, E> ConfigurerAdapterExt<ConfigurationData<S, E>, StateMachineConfigurationBuilder<S, E>>
    for DefaultSecurityConfigurer<S, E>
{
    fn configure(
        &mut self,
        builder: &mut StateMachineConfigurationBuilder<S, E>,
    ) -> Result<(), BoxError> {
        if self.enabled {
            // Set security enabled
            builder.set_security_enabled(true);

            // Set access decision managers
            if let Some(manager) = &self.transition_access_decision_manager {
                builder.set_transition_security_access_decision_manager(manager.clone());
            }

            if let Some(manager) = &self.event_access_decision_manager {
                builder.set_event_security_access_decision_manager(manager.clone());
            }

            // Set security rules
            if let Some(rule) = &self.event_security_rule {
                builder.set_event_security_rule(rule.clone());
            }

            if let Some(rule) = &self.transition_security_rule {
                builder.set_transition_security_rule(rule.clone());
            }
        }

        Ok(())
    }
}

impl<S, E> StateMachineSecurityConfigurer<S, E> for DefaultSecurityConfigurer<S, E> {
    fn enabled(&mut self, enabled: bool) -> &mut dyn StateMachineSecurityConfigurer<S, E> {
        self.enabled = enabled;
        self
    }

    fn transition_access_decision_manager(
        &mut self,
        manager: Arc<dyn AccessDecisionManager>,
    ) -> &mut dyn StateMachineSecurityConfigurer<S, E> {
        self.transition_access_decision_manager = Some(manager);
        self
    }

    fn event_access_decision_manager(
        &mut self,
        manager: Arc<dyn AccessDecisionManager>,
    ) -> &mut dyn StateMachineSecurityConfigurer<S, E> {
        self.event_access_decision_manager = Some(manager);
        self
    }

    fn event_with_attributes(
        &mut self,
        attributes: String,
        match_type: ComparisonType,
    ) -> &mut dyn StateMachineSecurityConfigurer<S, E> {
        if self.event_security_rule.is_none() {
            self.event_security_rule = Some(Default::default());
        }

        self.event_security_rule.as_mut().unwrap().set_attributes(
            SecurityRule::comma_delimited_list_to_security_attributes(&attributes),
        );

        self
    }

    fn event_with_expression(
        &mut self,
        expression: String,
    ) -> &mut dyn StateMachineSecurityConfigurer<S, E> {
        if self.event_security_rule.is_none() {
            self.event_security_rule = Some(Default::default());
        }

        self.event_security_rule
            .as_mut()
            .unwrap()
            .set_expression(expression);

        self
    }

    fn transition_with_attributes(
        &mut self,
        attributes: String,
        match_type: ComparisonType,
    ) -> &mut dyn StateMachineSecurityConfigurer<S, E> {
        if self.event_security_rule.is_none() {
            self.event_security_rule = Some(Default::default());
        }

        self.event_security_rule.as_mut().unwrap().set_attributes(
            SecurityRule::comma_delimited_list_to_security_attributes(&attributes),
        );

        self
    }

    fn transition_with_expression(
        &mut self,
        expression: String,
    ) -> &mut dyn StateMachineSecurityConfigurer<S, E> {
        if self.event_security_rule.is_none() {
            self.event_security_rule = Some(Default::default());
        }

        self.event_security_rule
            .as_mut()
            .unwrap()
            .set_expression(expression);

        self
    }
}

impl<S, E> Default for DefaultSecurityConfigurer<S, E> {
    fn default() -> Self {
        Self {
            enabled: true,
            transition_access_decision_manager: None,
            event_access_decision_manager: None,
            event_security_rule: None,
            transition_security_rule: None,

            base: Default::default(),
        }
    }
}
