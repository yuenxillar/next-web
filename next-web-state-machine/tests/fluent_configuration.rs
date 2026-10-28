//! Integration tests which build machines through the fluent configuration
//! chain.
//!
//! They cover the defect where the pending state of a configurer view was
//! dropped together with the view: every kind of transition is configured
//! through the chain and has to reach the built machine.

use std::sync::Arc;

use next_web_core::error::BoxError;
use next_web_core::messaging::generic_message::GenericMessage;
use next_web_state_machine::autoconfigure::state_machine_auto_configuration::StateMachineAutoConfiguration;
use next_web_state_machine::config::action::StateMachineAction;
use next_web_state_machine::config::builders::state_machine_config_builder::StateMachineConfigBuilder;
use next_web_state_machine::config::builders::state_machine_configuration_builder::StateMachineConfigurationBuilder;
use next_web_state_machine::config::builders::state_machine_configuration_configurer::StateMachineConfigurationConfigurer;
use next_web_state_machine::config::builders::state_machine_configurer::StateMachineConfigurer;
use next_web_state_machine::config::builders::state_machine_model_builder::StateMachineModelBuilder;
use next_web_state_machine::config::builders::state_machine_state_builder::StateMachineStateBuilder;
use next_web_state_machine::config::builders::state_machine_state_configurer::StateMachineStateConfigurer;
use next_web_state_machine::config::builders::state_machine_transition_builder::StateMachineTransitionBuilder;
use next_web_state_machine::config::builders::state_machine_transition_configurer::StateMachineTransitionConfigurer;
use next_web_state_machine::config::common::configurer::Configurer;
use next_web_state_machine::config::guard::StateMachineGuard;
use next_web_state_machine::config::state_machine_config::StateMachineConfig;
use next_web_state_machine::state_context::StateContext;
use next_web_state_machine::state_machine_event_result::ResultType;
use next_web_state_machine::transition::transition_kind::TransitionKind;

/// Implements the boilerplate a configurer needs on top of the four callbacks.
macro_rules! impl_configurer_plumbing {
    ($configurer:ty) => {
        impl Configurer<StateMachineConfig<i32, i32>, StateMachineConfigBuilder<i32, i32>>
            for $configurer
        {
            fn init(
                &mut self,
                _builder: &StateMachineConfigBuilder<i32, i32>,
            ) -> Result<(), BoxError> {
                Ok(())
            }

            fn configure(
                &mut self,
                _builder: &StateMachineConfigBuilder<i32, i32>,
            ) -> Result<(), BoxError> {
                Ok(())
            }

            fn is_assignable(&self, _builder: &StateMachineConfigBuilder<i32, i32>) -> bool {
                true
            }
        }
    };
}

/// Two states where `100` moves from `1` to `2` through an external transition.
#[derive(Clone)]
struct FlowConfigurer;

impl_configurer_plumbing!(FlowConfigurer);

impl StateMachineConfigurer<i32, i32> for FlowConfigurer {
    fn config_configure(
        &mut self,
        config: &mut StateMachineConfigurationBuilder<i32, i32>,
    ) -> Result<(), BoxError> {
        config
            .with_configuration()?
            .machine_id("flow".to_owned())
            .auto_startup(false);

        Ok(())
    }

    fn model_configure(
        &mut self,
        _model: &mut StateMachineModelBuilder<i32, i32>,
    ) -> Result<(), BoxError> {
        Ok(())
    }

    fn state_configure(
        &mut self,
        states: &mut StateMachineStateBuilder<i32, i32>,
    ) -> Result<(), BoxError> {
        states.with_states()?.initial(1).state(2);

        Ok(())
    }

    fn transition_configure(
        &mut self,
        transitions: &mut StateMachineTransitionBuilder<i32, i32>,
    ) -> Result<(), BoxError> {
        transitions
            .with_external()?
            .source(1)
            .target(2)
            .event(100);

        Ok(())
    }
}

/// Two states where `100` is an internal transition of `1`.
#[derive(Clone)]
struct InternalConfigurer;

impl_configurer_plumbing!(InternalConfigurer);

impl StateMachineConfigurer<i32, i32> for InternalConfigurer {
    fn config_configure(
        &mut self,
        config: &mut StateMachineConfigurationBuilder<i32, i32>,
    ) -> Result<(), BoxError> {
        config
            .with_configuration()?
            .machine_id("flow".to_owned())
            .auto_startup(false);

        Ok(())
    }

    fn model_configure(
        &mut self,
        _model: &mut StateMachineModelBuilder<i32, i32>,
    ) -> Result<(), BoxError> {
        Ok(())
    }

    fn state_configure(
        &mut self,
        states: &mut StateMachineStateBuilder<i32, i32>,
    ) -> Result<(), BoxError> {
        states.with_states()?.initial(1).state(2);

        Ok(())
    }

    fn transition_configure(
        &mut self,
        transitions: &mut StateMachineTransitionBuilder<i32, i32>,
    ) -> Result<(), BoxError> {
        transitions
            .with_internal()?
            .source(1)
            .event(100)
            .action(Arc::new(RecordingAction));

        Ok(())
    }
}

/// A guard which always denies, so a denied event proves the guard was attached.
struct DenyingGuard;

#[next_web_core::async_trait]
impl StateMachineGuard<i32, i32> for DenyingGuard {
    async fn evaluate(&self, _ctx: &dyn StateContext<i32, i32>) -> bool {
        false
    }
}

/// An action which records nothing, used to make an internal transition valid.
struct RecordingAction;

#[next_web_core::async_trait]
impl StateMachineAction<i32, i32> for RecordingAction {
    async fn execute(&self, _context: &dyn StateContext<i32, i32>) -> Result<(), BoxError> {
        Ok(())
    }
}

/// Two states where the only transition is blocked by a guard.
#[derive(Clone)]
struct GuardedConfigurer;

impl_configurer_plumbing!(GuardedConfigurer);

impl StateMachineConfigurer<i32, i32> for GuardedConfigurer {
    fn config_configure(
        &mut self,
        config: &mut StateMachineConfigurationBuilder<i32, i32>,
    ) -> Result<(), BoxError> {
        config
            .with_configuration()?
            .machine_id("flow".to_owned())
            .auto_startup(false);

        Ok(())
    }

    fn model_configure(
        &mut self,
        _model: &mut StateMachineModelBuilder<i32, i32>,
    ) -> Result<(), BoxError> {
        Ok(())
    }

    fn state_configure(
        &mut self,
        states: &mut StateMachineStateBuilder<i32, i32>,
    ) -> Result<(), BoxError> {
        states.with_states()?.initial(1).state(2);

        Ok(())
    }

    fn transition_configure(
        &mut self,
        transitions: &mut StateMachineTransitionBuilder<i32, i32>,
    ) -> Result<(), BoxError> {
        transitions
            .with_external()?
            .source(1)
            .event(100)
            .guard(Arc::new(DenyingGuard))
            .target(2);

        Ok(())
    }
}

#[tokio::test]
async fn the_fluent_chain_builds_an_external_transition() {
    let machine = StateMachineAutoConfiguration::build(FlowConfigurer).expect("the machine builds");

    machine.start().await.expect("the machine starts");
    assert_eq!(machine.state().map(|state| *state.id()), Some(1));

    let result = machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");

    assert_eq!(result.result_type(), ResultType::Accepted);
    assert_eq!(machine.state().map(|state| *state.id()), Some(2));
}

#[tokio::test]
async fn the_fluent_chain_builds_an_internal_transition() {
    let machine =
        StateMachineAutoConfiguration::build(InternalConfigurer).expect("the machine builds");

    let internal = machine
        .transitions()
        .into_iter()
        .filter(|transition| transition.kind() == TransitionKind::Internal)
        .count();

    assert_eq!(internal, 1, "the internal transition reached the machine");
}

#[tokio::test]
async fn the_fluent_chain_attaches_the_guard_and_the_target() {
    let machine =
        StateMachineAutoConfiguration::build(GuardedConfigurer).expect("the machine builds");

    machine.start().await.expect("the machine starts");

    let result = machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");

    assert_eq!(result.result_type(), ResultType::Denied);
    assert_eq!(
        machine.state().map(|state| *state.id()),
        Some(1),
        "a denying guard keeps the machine in the source state"
    );
}
