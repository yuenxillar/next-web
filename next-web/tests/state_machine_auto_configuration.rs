//! Integration test which builds a machine through the umbrella crate.

#![cfg(feature = "enable-state-machine")]

use std::sync::Arc;

use next_web::state_machine::StateMachine;
use next_web::state_machine::config::base_state_machine_factory::BaseStateMachineFactory;
use next_web::state_machine::config::model::configuration_data::ConfigurationData;
use next_web::state_machine::config::model::default_state_machine_model::DefaultStateMachineModel;
use next_web::state_machine::config::model::state_data::StateData;
use next_web::state_machine::config::model::transition_data::TransitionData;
use next_web::state_machine::config::model::transitions_data::TransitionsData;
use next_web::state_machine::config::state_machine_factory::StateMachineFactory;
use next_web::state_machine::region::Region;
use next_web::state_machine::state_machine_event_result::ResultType;
use next_web_core::messaging::generic_message::GenericMessage;

#[tokio::test]
async fn a_machine_built_through_the_umbrella_crate_handles_an_event() {
    // The configuration keeps its fields private, so the test uses the
    // constructor which exposes them.
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

    let factory = BaseStateMachineFactory::<i32, i32>::new(Arc::new(model), None);
    let machine = factory.get_state_machine().expect("the machine builds");

    assert_eq!(machine.id(), "flow");

    machine.start().await.expect("the machine starts");
    assert_eq!(machine.state().map(|state| *state.id()), Some(1));

    let result = machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");

    assert_eq!(result.result_type(), ResultType::Accepted);
    assert_eq!(machine.state().map(|state| *state.id()), Some(2));
}

/// A handler which the macro generates against the re-exported state machine.
#[next_web::on_transition(
    crate = next_web::state_machine,
    source = 1,
    target = 2,
    event = 100
)]
fn record_ship(
    _context: &dyn next_web::state_machine::state_context::StateContext<i32, i32>,
) -> Result<(), next_web_core::error::BoxError> {
    Ok(())
}

#[tokio::test]
async fn the_attribute_of_the_umbrella_crate_builds_a_handler() {
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
        Some("handler".to_owned()),
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

    let factory = BaseStateMachineFactory::<i32, i32>::new(Arc::new(model), None);
    let machine = factory.get_state_machine().expect("the machine builds");

    machine
        .add_transition_handler(Arc::new(record_ship_handler()))
        .expect("the handler is registered");

    machine.start().await.expect("the machine starts");
    machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");

    assert_eq!(machine.state().map(|state| *state.id()), Some(2));
}
