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
