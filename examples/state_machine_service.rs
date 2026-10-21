//! A service owns a machine, persists it when it is released and restores it
//! when it is acquired again.
//!
//! The machine moves from state 1 to state 2 on its own, with a timer, which is
//! what the persisted state captures.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use next_web::core::any::AnyValue;
use next_web::state_machine::extended_state::ExtendedState;
use next_web::state_machine::config::base_state_machine_factory::BaseStateMachineFactory;
use next_web::state_machine::config::model::configuration_data::ConfigurationData;
use next_web::state_machine::config::model::default_state_machine_model::DefaultStateMachineModel;
use next_web::state_machine::config::model::state_data::StateData;
use next_web::state_machine::config::model::transition_data::TransitionData;
use next_web::state_machine::config::model::transitions_data::TransitionsData;
use next_web::state_machine::persist::default_state_machine_persister::DefaultStateMachinePersister;
use next_web::state_machine::service::default_state_machine_service::DefaultStateMachineService;
use next_web::state_machine::service::state_machine_service::StateMachineService;
use next_web::state_machine::state_machine_context::StateMachineContext;
use next_web::state_machine::state_machine_persist::StateMachinePersist;
use next_web::state_machine::support::default_extended_state::DefaultExtendedState;
use next_web::state_machine::support::default_state_machine_context::DefaultStateMachineContext;
use next_web::state_machine::transition::transition_kind::TransitionKind;
use next_web_core::error::BoxError;

/// A store which keeps the state and the extended state of every machine.
#[derive(Default)]
struct MemoryStore {
    machines: Mutex<Vec<(String, i32, HashMap<String, AnyValue>)>>,
}

impl StateMachinePersist<i32, i32, String> for MemoryStore {
    fn write(
        &self,
        context: &dyn StateMachineContext<i32, i32>,
        machine_id: String,
    ) -> Result<(), BoxError> {
        let variables = context.extended_state().get_variables();

        let mut machines = self
            .machines
            .lock()
            .expect("the store lock is not poisoned");
        machines.retain(|(stored_id, _, _)| stored_id != &machine_id);
        machines.push((machine_id, *context.state(), variables));

        Ok(())
    }

    fn read(&self, machine_id: String) -> Result<Arc<dyn StateMachineContext<i32, i32>>, BoxError> {
        let machines = self
            .machines
            .lock()
            .expect("the store lock is not poisoned");

        let Some((id, state, variables)) = machines
            .iter()
            .find(|(stored_id, _, _)| stored_id == &machine_id)
            .cloned()
        else {
            // A store with nothing for the key reports MachineNotFound, which is
            // how the service knows that it has to build a fresh machine.
            return Err(next_web::state_machine::state_machine_error(
                next_web::state_machine::state_machine_exception::StateMachineErrorKind::MachineNotFound,
                "nothing was persisted",
            ));
        };

        let extended_state: Arc<dyn ExtendedState> =
            Arc::new(DefaultExtendedState::with_variables(variables));

        Ok(Arc::new(
            DefaultStateMachineContext::new(id, state).with_extended_state(extended_state),
        ))
    }
}

#[tokio::main]
async fn main() {
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
        Some("service".to_owned()),
        None,
        Vec::new(),
    );

    // The machine moves from 1 to 2 on its own after 20 milliseconds, and the
    // timer fires once.
    let model = DefaultStateMachineModel::new(
        configuration,
        Some(
            vec![
                StateData::<i32, i32>::with_initial(1, true),
                StateData::<i32, i32>::new(2),
            ]
            .into(),
        ),
        Some(TransitionsData::new(vec![TransitionData::with_all(
            1,
            2,
            None,
            None,
            Some(20),
            Some(1),
            Vec::new(),
            None,
            TransitionKind::External,
            None,
            "timer",
        )])),
    );

    let factory = Arc::new(BaseStateMachineFactory::<i32, i32>::new(Arc::new(model), None));

    let persister = DefaultStateMachinePersister::<i32, i32, String>::new(
        factory.clone(),
        Arc::new(MemoryStore::default()),
    );

    let service = DefaultStateMachineService::new(factory).with_persister(Arc::new(persister));

    let machine = service
        .acquire_state_machine("service")
        .await
        .expect("the service acquires the machine");

    // The timer fires after 20 milliseconds; wait for it instead of assuming a
    // margin, so a loaded machine does not make the example flaky.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);

    while machine.state().map(|state| *state.id()) != Some(2) {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the timer did not move the machine to state 2"
        );

        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    service
        .release_state_machine("service")
        .await
        .expect("the service persists and releases the machine");

    let restored = service
        .acquire_state_machine("service")
        .await
        .expect("the service acquires the machine again");

    let state = restored
        .state()
        .map(|state| *state.id())
        .expect("the restored machine has a current state");

    assert_eq!(
        state, 2,
        "the restored machine continues in the persisted state"
    );

    println!("restored: {state}");

    service
        .release_state_machine("service")
        .await
        .expect("the service releases the machine");
}
