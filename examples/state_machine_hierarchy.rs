//! A parent machine forwards an event to the machine of its current state.
//!
//! Building the machines directly is fine for an example; an application usually
//! lets a `StateMachineService` own them instead.

use std::sync::Arc;

use next_web::state_machine::config::base_state_machine_factory::BaseStateMachineFactory;
use next_web::state_machine::config::model::configuration_data::ConfigurationData;
use next_web::state_machine::config::model::default_state_machine_model::DefaultStateMachineModel;
use next_web::state_machine::config::model::state_data::StateData;
use next_web::state_machine::config::model::transition_data::TransitionData;
use next_web::state_machine::config::model::transitions_data::TransitionsData;
use next_web::state_machine::config::state_machine_factory::StateMachineFactory;
use next_web::state_machine::state_machine_event_result::ResultType;
use next_web_core::messaging::generic_message::GenericMessage;

/// Builds the configuration of one machine.
fn configuration(machine_id: &str) -> ConfigurationData<i32, i32> {
    ConfigurationData::with_parameters(
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
        Some(machine_id.to_owned()),
        None,
        Vec::new(),
    )
}

#[tokio::main]
async fn main() {
    // The child machine moves from 21 to 22 on the event 200.
    let child_model = DefaultStateMachineModel::new(
        configuration("child"),
        Some(
            vec![
                StateData::<i32, i32>::with_initial(21, true),
                StateData::<i32, i32>::new(22),
            ]
            .into(),
        ),
        Some(TransitionsData::new(vec![TransitionData::new(21, 22, 200)])),
    );

    let child = BaseStateMachineFactory::<i32, i32>::new(Arc::new(child_model), None)
        .get_state_machine()
        .expect("the child machine builds from its configuration");

    // The parent moves from 1 to 2 on the event 100, and the state 2 owns the
    // child machine.
    // The state 2 owns the child machine. The model holds that one instance, so a
    // model which builds the parent more than once uses
    // `StateData::set_submachine_factory` to give every parent a fresh child.
    let mut parent_state_2 = StateData::<i32, i32>::new(2);
    parent_state_2.set_submachine(child.clone());

    let parent_model = DefaultStateMachineModel::new(
        configuration("parent"),
        Some(
            vec![StateData::<i32, i32>::with_initial(1, true), parent_state_2].into(),
        ),
        Some(TransitionsData::new(vec![TransitionData::new(1, 2, 100)])),
    );

    let parent = BaseStateMachineFactory::<i32, i32>::new(Arc::new(parent_model), None)
        .get_state_machine()
        .expect("the parent machine builds from its configuration");

    parent.start().await.expect("the parent machine starts");

    parent
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");

    assert_eq!(
        parent.state().map(|state| *state.id()),
        Some(2),
        "the parent enters the state which owns the child"
    );

    let result = parent
        .send_event(Arc::new(GenericMessage::with_payload(200)))
        .await
        .expect("the event is sent");

    assert_eq!(
        result.result_type(),
        ResultType::Accepted,
        "the parent forwards the event to its child"
    );

    assert_eq!(
        parent.state().map(|state| *state.id()),
        Some(2),
        "the parent stays in the state which owns the child"
    );

    assert_eq!(
        child.state().map(|state| *state.id()),
        Some(22),
        "the child took the forwarded transition"
    );

    println!("child moved: true");

    parent.stop().await.expect("the parent machine stops");

    // Stopping the parent does not exit its current state, so the child keeps
    // running and the application which owns it stops it.
    child.stop().await.expect("the child machine stops");
}
