//! A service owns a machine, persists it when it is released and restores it
//! when it is acquired again.
//!
//! The machine moves from state 1 to state 2 on its own, with a timer, which is
//! what the persisted state captures.

use std::sync::Arc;
use std::sync::Mutex;

use next_web::state_machine::StateMachine;
use next_web::state_machine::config::base_state_machine_factory::BaseStateMachineFactory;
use next_web::state_machine::config::model::configuration_data::ConfigurationData;
use next_web::state_machine::config::model::default_state_machine_model::DefaultStateMachineModel;
use next_web::state_machine::config::model::state_data::StateData;
use next_web::state_machine::config::model::transition_data::TransitionData;
use next_web::state_machine::config::model::transitions_data::TransitionsData;
use next_web::state_machine::config::state_machine_factory::StateMachineFactory;
use next_web::state_machine::persist::default_state_machine_persister::DefaultStateMachinePersister;
use next_web::state_machine::region::Region;
use next_web::state_machine::service::default_state_machine_service::DefaultStateMachineService;
use next_web::state_machine::service::state_machine_service::StateMachineService;
use next_web::state_machine::state_machine_context::StateMachineContext;
use next_web::state_machine::state_machine_persist::StateMachinePersist;
use next_web::state_machine::support::default_state_machine_context::DefaultStateMachineContext;
use next_web::state_machine::transition::transition_kind::TransitionKind;
use next_web_core::error::BoxError;

/// A store which keeps the state of every machine by identifier.
#[derive(Default)]
struct MemoryStore {
    states: Mutex<Vec<(String, i32)>>,
}

impl StateMachinePersist<i32, i32, String> for MemoryStore {
    fn write(
        &self,
        context: &dyn StateMachineContext<i32, i32>,
        machine_id: String,
    ) -> Result<(), BoxError> {
        let mut states = self.states.lock().expect("the store lock is not poisoned");
        states.retain(|(stored_id, _)| stored_id != &machine_id);
        states.push((machine_id, *context.state()));

        Ok(())
    }

    fn read(&self, machine_id: String) -> Result<Arc<dyn StateMachineContext<i32, i32>>, BoxError> {
        let states = self.states.lock().expect("the store lock is not poisoned");

        let Some((id, state)) = states
            .iter()
            .find(|(stored_id, _)| stored_id == &machine_id)
            .cloned()
        else {
            // A store with nothing for the key reports MachineNotFound, which is
            // how the service knows that it has to build a fresh machine.
            return Err(next_web::state_machine::state_machine_error(
                next_web::state_machine::state_machine_exception::StateMachineErrorKind::MachineNotFound,
                "nothing was persisted",
            ));
        };

        Ok(Arc::new(DefaultStateMachineContext::new(id, state)))
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

    tokio::time::sleep(std::time::Duration::from_millis(60)).await;

    assert_eq!(
        machine.state().map(|state| *state.id()),
        Some(2),
        "the timer moved the machine before it was released"
    );

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
