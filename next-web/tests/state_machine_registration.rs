//! An application declares a machine with a configuration and injects it.

#![cfg(feature = "enable-state-machine")]

use std::sync::Arc;

use next_web::core::error::BoxError;
use next_web::state_machine::StateMachine;
use next_web::state_machine::config::builders::state_machine_config_builder::StateMachineConfigBuilder;
use next_web::state_machine::config::builders::state_machine_configuration_builder::StateMachineConfigurationBuilder;
use next_web::state_machine::config::builders::state_machine_configuration_configurer::StateMachineConfigurationConfigurer;
use next_web::state_machine::config::builders::state_machine_configurer::StateMachineConfigurer;
use next_web::state_machine::config::builders::state_machine_model_builder::StateMachineModelBuilder;
use next_web::state_machine::config::builders::state_machine_state_builder::StateMachineStateBuilder;
use next_web::state_machine::config::builders::state_machine_state_configurer::StateMachineStateConfigurer;
use next_web::state_machine::config::builders::state_machine_transition_builder::StateMachineTransitionBuilder;
use next_web::state_machine::config::builders::state_machine_transition_configurer::StateMachineTransitionConfigurer;
use next_web::state_machine::config::common::configurer::Configurer;
use next_web::state_machine::config::state_machine_config::StateMachineConfig;
use next_web::state_machine::state_machine_event_result::ResultType;
use next_web_context::ApplicationContextExt;
use next_web_core::messaging::generic_message::GenericMessage;
use next_web::macros::statemachine::state_machine;

/// A configuration which contributes a two state machine.
#[derive(Clone)]
struct TestStateMachineConfig;

#[state_machine(name = "orderStateMachine")]
impl StateMachineConfigurer<i32, i32> for TestStateMachineConfig {
    fn config_configure(
        &mut self,
        config: &mut StateMachineConfigurationBuilder<i32, i32>,
    ) -> Result<(), BoxError> {
        config
            .with_configuration()?
            .machine_id(String::from("orderStateMachine"));

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
        states.with_states()?.initial(1).states(vec![1, 2]);

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

impl Configurer<StateMachineConfig<i32, i32>, StateMachineConfigBuilder<i32, i32>>
    for TestStateMachineConfig
{
    fn init(&mut self, _builder: &StateMachineConfigBuilder<i32, i32>) -> Result<(), BoxError> {
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

/// A persister which counts the machines it wrote.
#[derive(Default)]
struct CountingPersister {
    writes: Arc<std::sync::atomic::AtomicUsize>,
}

#[next_web_core::async_trait]
impl
    next_web::state_machine::persist::state_machine_persister::StateMachinePersister<
        i32,
        i32,
        String,
    > for CountingPersister
{
    async fn persist(
        &self,
        _state_machine: Arc<dyn StateMachine<i32, i32>>,
        _context: String,
    ) -> Result<(), BoxError> {
        self.writes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        Ok(())
    }

    async fn restore(&self, _context: String) -> Result<Arc<dyn StateMachine<i32, i32>>, BoxError> {
        Err(next_web::state_machine::state_machine_error(
            next_web::state_machine::state_machine_exception::StateMachineErrorKind::MachineNotFound,
            "no persisted state exists",
        ))
    }
}

/// The configuration of the test is picked up by the framework and its machine is
/// injectable by type and by name, without any registration code in the test.
#[tokio::test]
async fn the_machine_of_a_configuration_is_registered_and_injects() {
    let mut context = next_web::context::DefaultApplicationContext::default();

    let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let persister = Arc::new(CountingPersister {
        writes: writes.clone(),
    });

    context.insert_singleton::<Arc<
        dyn next_web::state_machine::persist::state_machine_persister::StateMachinePersister<
                i32,
                i32,
                String,
            >,
    >>(persister);

    next_web::autoregister::state_machine_autoregister::apply_state_machines(&mut context)
        .await
        .expect("the machines of the application are registered");

    let machines = context.resolve_by_type::<Arc<dyn StateMachine<i32, i32>>>();

    assert_eq!(
        machines.len(),
        1,
        "the machine of the configuration is registered"
    );

    let machine = machines[0].clone();

    assert_eq!(
        machine.id(),
        "orderStateMachine",
        "the machine carries the id its configuration declares"
    );

    let named = context.resolve_by_type_with_names::<Arc<dyn StateMachine<i32, i32>>>();

    let names: Vec<String> = named.iter().map(|(name, _)| name.clone()).collect();

    assert!(
        names.iter().any(|name| name == "orderStateMachine"),
        "the machine is registered under its bean name, got: {names:?}"
    );

    machine.start().await.expect("the machine starts");

    let result = machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");

    assert_eq!(result.result_type(), ResultType::Accepted);
    assert_eq!(machine.state().map(|state| *state.id()), Some(2));

    // A shutdown persists every machine which a persister stores, before it
    // stops it.
    next_web::autoregister::state_machine_autoregister::stop_all_registered_machines()
        .await
        .expect("every machine is stopped");

    assert_eq!(
        writes.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "the machine was persisted once before it stopped"
    );
    assert!(!machine.is_running(), "the machine is stopped");
}
