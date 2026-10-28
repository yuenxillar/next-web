//! Security enforced through the interceptor chain.
//!
//! The Java original splits this into a `StateMachineSecurityInterceptor` plus
//! four voters and two expression handlers, because Spring Security evaluates
//! SpEL expressions. The Rust port enforces attributes with a
//! [`AccessDecisionManager`](crate::security::access_decision_manager::AccessDecisionManager)
//! and expressions with a pluggable
//! [`ExpressionEvaluator`](crate::security::expression_evaluator::ExpressionEvaluator).

use std::marker::PhantomData;
use std::sync::Arc;

use next_web_core::any::AnyValue;
use next_web_core::error::BoxError;
use next_web_core::messaging::Message;
use tracing::debug;

use crate::StateMachine;
use crate::security::access_decision_manager::{
    AccessDecisionManager, ConfigAttribute, SimpleConfigAttribute, authorities_of_authentication,
};
use crate::security::expression_evaluator::ExpressionEvaluator;
use crate::security::security_rule::SecurityRule;
use crate::state::StateMachineState;
use crate::state_context::StateContext;
use crate::state_machine_exception::{StateMachineErrorKind, state_machine_error};
use crate::support::state_machine_interceptor::StateMachineInterceptor;
use crate::transition::StateMachineTransition;

/// The message header which carries the authentication of an event.
pub const HEADER_AUTHENTICATION: &str = "STATEMACHINE_AUTHENTICATION";

/// Enforces the security rules of events and transitions.
///
/// The authentication is read from the [`HEADER_AUTHENTICATION`] header and may
/// be a single authority, a list of authorities or a `HashSet<String>`.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct DefaultStateMachineSecurityInterceptor<S, E> {
    event_rule: Option<SecurityRule>,
    transition_rule: Option<SecurityRule>,
    event_decision_manager: Option<Arc<dyn AccessDecisionManager>>,
    transition_decision_manager: Option<Arc<dyn AccessDecisionManager>>,
    expression_evaluator: Option<Arc<dyn ExpressionEvaluator>>,
    _marker: PhantomData<(S, E)>,
}

impl<S, E> DefaultStateMachineSecurityInterceptor<S, E> {
    /// Creates a new security interceptor.
    ///
    /// # Arguments
    /// * `event_rule` - the rule applied to events
    /// * `transition_rule` - the rule applied to transitions
    pub fn new(
        event_rule: Option<SecurityRule>,
        transition_rule: Option<SecurityRule>,
    ) -> Self {
        Self {
            event_rule,
            transition_rule,
            event_decision_manager: None,
            transition_decision_manager: None,
            expression_evaluator: None,
            _marker: PhantomData,
        }
    }

    /// Sets the decision managers used for attribute rules.
    ///
    /// # Arguments
    /// * `event_decision_manager` - the manager for event rules
    /// * `transition_decision_manager` - the manager for transition rules
    pub fn with_decision_managers(
        mut self,
        event_decision_manager: Option<Arc<dyn AccessDecisionManager>>,
        transition_decision_manager: Option<Arc<dyn AccessDecisionManager>>,
    ) -> Self {
        self.event_decision_manager = event_decision_manager;
        self.transition_decision_manager = transition_decision_manager;
        self
    }

    /// Sets the evaluator used for expression rules.
    ///
    /// # Arguments
    /// * `expression_evaluator` - the evaluator for expression rules
    pub fn with_expression_evaluator(
        mut self,
        expression_evaluator: Arc<dyn ExpressionEvaluator>,
    ) -> Self {
        self.expression_evaluator = Some(expression_evaluator);
        self
    }

    /// Checks whether this interceptor has a rule to enforce.
    pub fn has_rules(&self) -> bool {
        self.event_rule.is_some() || self.transition_rule.is_some()
    }
}

impl<S, E> DefaultStateMachineSecurityInterceptor<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Evaluates a rule against the authentication of a message.
    ///
    /// # Arguments
    /// * `rule` - the rule to enforce
    /// * `authentication` - the authentication which grants the authorities
    /// * `decision_manager` - the manager which decides attribute rules
    /// * `operation` - the operation reported in the error
    fn check(
        &self,
        rule: &SecurityRule,
        authentication: Option<&AnyValue>,
        decision_manager: Option<&dyn AccessDecisionManager>,
        operation: &str,
    ) -> Result<(), BoxError> {
        if let Some(required) = rule.attributes() {
            if let Some(manager) = decision_manager {
                let attributes: Vec<Box<dyn ConfigAttribute>> = required
                    .iter()
                    .map(|attribute| {
                        Box::new(SimpleConfigAttribute::new(attribute.clone()))
                            as Box<dyn ConfigAttribute>
                    })
                    .collect();

                let authentication = authentication
                    .cloned()
                    .unwrap_or_else(|| AnyValue::from(""));

                return manager.decide(&authentication, &authentication, attributes);
            }

            let authorities = authentication
                .map(authorities_of_authentication)
                .unwrap_or_default();

            return if rule.comparison_type().evaluate_access(required, &authorities) {
                Ok(())
            } else {
                Err(state_machine_error(
                    StateMachineErrorKind::AccessDenied,
                    format!("the security rule denied the {operation}"),
                ))
            };
        }

        if let Some(expression) = rule.expression() {
            let Some(evaluator) = self.expression_evaluator.as_ref() else {
                debug!("a security expression was configured without an evaluator");
                return Err(state_machine_error(
                    StateMachineErrorKind::AccessDenied,
                    format!(
                        "the security expression of the {operation} cannot be evaluated because no expression evaluator is configured"
                    ),
                ));
            };

            let authentication = authentication
                .cloned()
                .unwrap_or_else(|| AnyValue::from(""));

            return match evaluator.evaluate(expression, &authentication)? {
                true => Ok(()),
                false => Err(state_machine_error(
                    StateMachineErrorKind::AccessDenied,
                    format!("the security expression denied the {operation}"),
                )),
            };
        }

        Ok(())
    }
}

impl<S, E> StateMachineInterceptor<S, E> for DefaultStateMachineSecurityInterceptor<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn pre_event(
        &self,
        message: Arc<dyn Message<E>>,
        _state_machine: &dyn StateMachine<S, E>,
    ) -> Result<Arc<dyn Message<E>>, BoxError> {
        let Some(rule) = self.event_rule.as_ref() else {
            return Ok(message);
        };

        let authentication = message.get_headers().get(HEADER_AUTHENTICATION);
        self.check(
            rule,
            authentication,
            self.event_decision_manager.as_deref(),
            "event",
        )?;

        Ok(message)
    }

    fn pre_state_change(
        &self,
        _state: &dyn StateMachineState<S, E>,
        _message: Option<&dyn Message<E>>,
        _transition: Option<&dyn StateMachineTransition<S, E>>,
        _state_machine: &dyn StateMachine<S, E>,
        _root_state_machine: &dyn StateMachine<S, E>,
    ) -> Result<(), BoxError> {
        Ok(())
    }

    fn post_state_change(
        &self,
        _state: &dyn StateMachineState<S, E>,
        _message: Option<&dyn Message<E>>,
        _transition: Option<&dyn StateMachineTransition<S, E>>,
        _state_machine: &dyn StateMachine<S, E>,
        _root_state_machine: &dyn StateMachine<S, E>,
    ) {
    }

    fn pre_transition(&self, state_context: &dyn StateContext<S, E>) -> Result<(), BoxError> {
        let Some(rule) = self.transition_rule.as_ref() else {
            return Ok(());
        };

        let authentication = state_context.message_header(HEADER_AUTHENTICATION);
        self.check(
            rule,
            authentication,
            self.transition_decision_manager.as_deref(),
            "transition",
        )
    }

    fn post_transition(&self, _state_context: &dyn StateContext<S, E>) -> Result<(), BoxError> {
        Ok(())
    }

    fn state_machine_error(
        &self,
        _state_machine: &dyn StateMachine<S, E>,
        error: BoxError,
    ) -> Option<BoxError> {
        Some(error)
    }
}
