//! Tests of the transition handlers.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use next_web_core::error::BoxError;
use next_web_core::messaging::generic_message::GenericMessage;
use next_web_state_machine::config::base_state_machine_factory::BaseStateMachineFactory;
use next_web_state_machine::config::model::configuration_data::ConfigurationData;
use next_web_state_machine::config::model::default_state_machine_model::DefaultStateMachineModel;
use next_web_state_machine::config::model::state_data::StateData;
use next_web_state_machine::config::model::transition_data::TransitionData;
use next_web_state_machine::config::model::transitions_data::TransitionsData;
use next_web_state_machine::config::state_machine_factory::StateMachineFactory;
use next_web_state_machine::handler::on_transition::OnTransition;
use next_web_state_machine::state_context::StateContext;
use next_web_state_machine::state_machine_event_result::ResultType;

/// Builds a machine which moves `1 -> 2` on `100` and `2 -> 3` on `200`.
fn build_machine(
    handler: OnTransition<i32, i32>,
) -> Arc<dyn next_web_state_machine::StateMachine<i32, i32>> {
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
        .expect("the machine builds");

    machine
        .add_transition_handler(Arc::new(handler))
        .expect("the handler is registered");

    machine
}

#[tokio::test]
async fn a_handler_runs_only_for_the_matching_transition() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();

    let handler = OnTransition::<i32, i32>::new(Arc::new(move |_context: &dyn StateContext<i32, i32>| {
        counter.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }))
    .target(2);

    let machine = build_machine(handler);
    machine.start().await.expect("the machine starts");

    machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");
    assert_eq!(calls.load(Ordering::SeqCst), 1, "the handler ran once");

    machine
        .send_event(Arc::new(GenericMessage::with_payload(200)))
        .await
        .expect("the event is sent");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "the handler ignores the transition into state 3"
    );
    assert_eq!(machine.state().map(|state| *state.id()), Some(3));
}

#[tokio::test]
async fn a_handler_can_filter_by_event_and_source() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();

    let handler = OnTransition::<i32, i32>::new(Arc::new(move |_context: &dyn StateContext<i32, i32>| {
        counter.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }))
    .source(2)
    .event(200);

    let machine = build_machine(handler);
    machine.start().await.expect("the machine starts");

    machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "the handler ignores the first transition"
    );

    machine
        .send_event(Arc::new(GenericMessage::with_payload(200)))
        .await
        .expect("the event is sent");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "the handler runs for the transition out of state 2"
    );
}

#[tokio::test]
async fn a_handler_failure_reaches_the_caller() {    let handler = OnTransition::<i32, i32>::new(Arc::new(
        |_context: &dyn StateContext<i32, i32>| {
            Err(Box::new(std::io::Error::other("the handler failed")) as BoxError)
        },
    ));

    let machine = build_machine(handler);
    machine.start().await.expect("the machine starts");

    let result = machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await;

    if let Ok(result) = result {
        assert_ne!(
            result.result_type(),
            ResultType::Accepted,
            "a failed handler does not report an accepted event"
        );
    }
}

// A project which depends on the state machine crate directly names it, because
// the macro targets the re-export of the umbrella crate by default.
#[next_web_macros::on_transition(crate = next_web_state_machine, source = 1, target = 2, event = 100)]
fn ship(context: &dyn StateContext<i32, i32>) -> Result<(), BoxError> {
    let _ = context;

    Ok(())
}

#[tokio::test]
async fn the_attribute_builds_the_handler() {
    let machine = build_machine(ship_handler());
    machine.start().await.expect("the machine starts");

    machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");
    assert_eq!(machine.state().map(|state| *state.id()), Some(2));
}
