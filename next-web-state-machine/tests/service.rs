//! Tests of the service layer.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use next_web_core::error::BoxError;
use next_web_core::messaging::generic_message::GenericMessage;
use next_web_state_machine::StateMachine;
use next_web_state_machine::config::base_state_machine_factory::BaseStateMachineFactory;
use next_web_state_machine::config::model::configuration_data::ConfigurationData;
use next_web_state_machine::config::model::default_state_machine_model::DefaultStateMachineModel;
use next_web_state_machine::config::model::state_data::StateData;
use next_web_state_machine::config::model::transition_data::TransitionData;
use next_web_state_machine::config::model::transitions_data::TransitionsData;
use next_web_state_machine::config::state_machine_factory::StateMachineFactory;
use next_web_state_machine::persist::state_machine_persister::StateMachinePersister;
use next_web_state_machine::service::default_state_machine_service::DefaultStateMachineService;
use next_web_state_machine::service::state_machine_service::StateMachineService;
use next_web_state_machine::state_machine_error;
use next_web_state_machine::state_machine_event_result::ResultType;
use next_web_state_machine::state_machine_exception::StateMachineErrorKind;

/// Builds a factory whose machine moves from `1` to `2` on the event `100`.
fn build_factory() -> Arc<BaseStateMachineFactory<i32, i32>> {
    let configuration = ConfigurationData::<i32, i32>::with_parameters(
        false,
        None,
        Vec::new(),
        false,
        None,
        None,
        None,
        None,
        false,
        None,
        Some("flow".to_owned()),
        None,
        Vec::new(),
    );

    let model = DefaultStateMachineModel::new(
        configuration,
        Some(
            vec![
                StateData::<i32, i32>::with_initial(1, true),
                StateData::<i32, i32>::new(2),
            ]
            .into(),
        ),
        Some(TransitionsData::new(vec![TransitionData::new(1, 2, 100)])),
    );

    Arc::new(BaseStateMachineFactory::<i32, i32>::new(Arc::new(model), None))
}

/// A store which keeps the current state of every machine by identifier.
struct MemoryPersister {
    factory: Arc<dyn StateMachineFactory<i32, i32>>,
    states: Mutex<HashMap<String, i32>>,
}

impl MemoryPersister {
    fn new(factory: Arc<dyn StateMachineFactory<i32, i32>>) -> Self {
        Self {
            factory,
            states: Mutex::new(HashMap::new()),
        }
    }
}

#[next_web_core::async_trait]
impl StateMachinePersister<i32, i32, String> for MemoryPersister {
    async fn persist(
        &self,
        state_machine: Arc<dyn StateMachine<i32, i32>>,
        context: String,
    ) -> Result<(), BoxError> {
        let state = state_machine.state().ok_or_else(|| {
            state_machine_error(
                StateMachineErrorKind::NotStarted,
                "the machine has no state to persist",
            )
        })?;

        self.states
            .lock()
            .expect("lock")
            .insert(context, *state.id());

        Ok(())
    }

    async fn restore(
        &self,
        context: String,
    ) -> Result<Arc<dyn StateMachine<i32, i32>>, BoxError> {
        let state = self.states.lock().expect("lock").get(&context).copied();

        let Some(state) = state else {
            return Err(state_machine_error(
                StateMachineErrorKind::MachineNotFound,
                "no persisted state exists",
            ));
        };

        let machine = self.factory.get_state_machine_with_id(context)?;
        machine.reset_to(&state)?;

        Ok(machine)
    }
}

fn service() -> DefaultStateMachineService<i32, i32> {
    DefaultStateMachineService::new(build_factory())
}

fn service_with_persister() -> DefaultStateMachineService<i32, i32> {
    let factory = build_factory();

    DefaultStateMachineService::new(factory.clone())
        .with_persister(Arc::new(MemoryPersister::new(factory)))
}

#[tokio::test]
async fn acquire_returns_the_same_started_machine() {
    let service = service();

    let first = service
        .acquire_state_machine("flow")
        .await
        .expect("the machine is acquired");
    let second = service
        .acquire_state_machine("flow")
        .await
        .expect("the machine is acquired again");

    assert!(
        Arc::ptr_eq(&first, &second),
        "the service hands out the machine it built"
    );
    assert!(first.is_running(), "the machine is started");
    assert_eq!(first.state().map(|state| *state.id()), Some(1));
}

#[tokio::test]
async fn release_stops_the_machine_and_a_new_acquire_builds_a_fresh_one() {
    let service = service();

    let first = service
        .acquire_state_machine("flow")
        .await
        .expect("the machine is acquired");
    service
        .release_state_machine("flow")
        .await
        .expect("the machine is released");

    assert!(!first.is_running(), "the released machine is stopped");
    assert!(first.state().is_none(), "a stopped machine has no state");

    let second = service
        .acquire_state_machine("flow")
        .await
        .expect("the machine is acquired again");

    assert!(
        !Arc::ptr_eq(&first, &second),
        "the released machine is not handed out anymore"
    );
    assert_eq!(second.state().map(|state| *state.id()), Some(1));
}

#[tokio::test]
async fn acquire_restores_the_persisted_state() {
    let service = service_with_persister();

    let machine = service
        .acquire_state_machine("flow")
        .await
        .expect("the machine is acquired");
    machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");
    assert_eq!(machine.state().map(|state| *state.id()), Some(2));

    service
        .release_state_machine("flow")
        .await
        .expect("the machine is released and persisted");

    let restored = service
        .acquire_state_machine("flow")
        .await
        .expect("the machine is acquired again");

    assert_eq!(
        restored.state().map(|state| *state.id()),
        Some(2),
        "the machine continues in the state which was released"
    );

    let result = restored
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");
    assert_eq!(result.result_type(), ResultType::Denied);
}

#[tokio::test]
async fn acquire_without_start_leaves_the_machine_stopped() {
    let service = service();

    let machine = service
        .acquire_state_machine_with_start("flow", false)
        .await
        .expect("the machine is acquired");

    assert!(!machine.is_running(), "the machine is not started");
    assert!(machine.state().is_none());

    // The next acquire with start returns the cached machine and starts it.
    let started = service
        .acquire_state_machine("flow")
        .await
        .expect("the machine is acquired again");

    assert!(Arc::ptr_eq(&machine, &started));
    assert!(started.is_running());
    assert_eq!(started.state().map(|state| *state.id()), Some(1));
}

#[tokio::test]
async fn releasing_twice_is_idempotent() {
    let service = service();

    service
        .acquire_state_machine("flow")
        .await
        .expect("the machine is acquired");

    service
        .release_state_machine("flow")
        .await
        .expect("the machine is released");
    service
        .release_state_machine("flow")
        .await
        .expect("releasing again is not an error");
}

#[tokio::test]
async fn acquire_builds_a_fresh_machine_when_nothing_was_persisted() {
    let service = service_with_persister();

    let machine = service
        .acquire_state_machine("flow")
        .await
        .expect("the machine is built because the store is empty");

    assert!(machine.is_running());
    assert_eq!(machine.state().map(|state| *state.id()), Some(1));
}
