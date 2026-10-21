//! Reproduction of the fluent configuration defect.
//!
//! The configurer chain is the documented way to build a machine, but the last
//! transition configured through `with_external()` is dropped: the configurer it
//! returns is a builder view whose pending transition lives in the view, so it
//! disappears together with the view. The test is ignored until the pending
//! state is shared through the builder.

use std::sync::Arc;

use next_web_core::error::BoxError;
use next_web_core::messaging::generic_message::GenericMessage;
use next_web_state_machine::autoconfigure::state_machine_auto_configuration::StateMachineAutoConfiguration;
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
use next_web_state_machine::config::state_machine_config::StateMachineConfig;
use next_web_state_machine::state_machine_event_result::ResultType;

/// Two states where `100` moves from `1` to `2`.
#[derive(Clone)]
struct FlowConfigurer;

impl Configurer<StateMachineConfig<i32, i32>, StateMachineConfigBuilder<i32, i32>>
    for FlowConfigurer
{
    fn init(&mut self, _builder: &StateMachineConfigBuilder<i32, i32>) -> Result<(), BoxError> {
        Ok(())
    }

    fn configure(&mut self, _builder: &StateMachineConfigBuilder<i32, i32>) -> Result<(), BoxError> {
        Ok(())
    }

    fn is_assignable(&self, _builder: &StateMachineConfigBuilder<i32, i32>) -> bool {
        true
    }
}

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
        transitions.with_external()?.source(1).target(2).event(100);

        Ok(())
    }
}

#[tokio::test]
#[ignore = "the pending transition of a configurer view is dropped when the view is dropped"]
async fn the_fluent_configuration_chain_builds_a_transition() {
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
