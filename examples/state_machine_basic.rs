//! The smallest machine: three states and two events.
//!
//! Building the machine directly is fine for an example; an application usually
//! lets a `StateMachineService` own the machine instead.

use std::sync::Arc;

use next_web::state_machine::config::base_state_machine_factory::BaseStateMachineFactory;
use next_web::state_machine::config::model::configuration_data::ConfigurationData;
use next_web::state_machine::config::model::default_state_machine_model::DefaultStateMachineModel;
use next_web::state_machine::config::model::state_data::StateData;
use next_web::state_machine::config::model::transition_data::TransitionData;
use next_web::state_machine::config::model::transitions_data::TransitionsData;
use next_web::state_machine::config::state_machine_factory::StateMachineFactory;
use next_web_core::messaging::generic_message::GenericMessage;

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
        Some("basic".to_owned()),
        None,
        Vec::new(),
    );

    let model = DefaultStateMachineModel::new(
        configuration,
        Some(
            vec![
                StateData::<i32, i32>::with_initial(1, true),
                StateData::<i32, i32>::new(2),
                StateData::<i32, i32>::new(3),
            ]
            .into(),
        ),
        Some(TransitionsData::new(vec![
            TransitionData::new(1, 2, 100),
            TransitionData::new(2, 3, 200),
        ])),
    );

    let machine = BaseStateMachineFactory::<i32, i32>::new(Arc::new(model), None)
        .get_state_machine()
        .expect("the machine builds from its configuration");

    machine.start().await.expect("the machine starts");

    assert_eq!(
        machine.state().map(|state| *state.id()),
        Some(1),
        "the initial state is entered on start"
    );

    machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");

    assert_eq!(
        machine.state().map(|state| *state.id()),
        Some(2),
        "the event moves the machine to state 2"
    );

    machine
        .send_event(Arc::new(GenericMessage::with_payload(200)))
        .await
        .expect("the event is sent");

    assert_eq!(
        machine.state().map(|state| *state.id()),
        Some(3),
        "the second event moves the machine to state 3"
    );

    let state = machine
        .state()
        .map(|state| *state.id())
        .expect("the machine has a current state");

    println!("state: {state}");

    machine.stop().await.expect("the machine stops");
}
