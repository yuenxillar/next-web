//! Concurrency and lifecycle tests.
//!
//! A machine is shared between tasks, so the tests drive one machine from
//! several tasks and check that the event processing stays serialized, that the
//! lifecycle transitions are safe under contention and that stopping the machine
//! disarms its triggers.

use std::sync::Arc;
use std::time::Duration;

use next_web_core::messaging::generic_message::GenericMessage;
use next_web_state_machine::StateMachine;
use next_web_state_machine::config::base_state_machine_factory::BaseStateMachineFactory;
use next_web_state_machine::config::model::configuration_data::ConfigurationData;
use next_web_state_machine::config::model::default_state_machine_model::DefaultStateMachineModel;
use next_web_state_machine::config::model::state_data::StateData;
use next_web_state_machine::config::model::transition_data::TransitionData;
use next_web_state_machine::config::model::transitions_data::TransitionsData;
use next_web_state_machine::config::state_machine_factory::StateMachineFactory;
use next_web_state_machine::state_machine_event_result::ResultType;

/// Builds a machine which moves from `1` to `2` on the event `100`.
fn build_two_state_machine() -> Arc<dyn StateMachine<i32, i32>> {
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
        Some("plain".to_owned()),
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

    BaseStateMachineFactory::<i32, i32>::new(Arc::new(model), None)
        .get_state_machine()
        .expect("the machine builds")
}

/// Builds a machine which moves from `1` to `2` when its timer fires.
fn build_timed_machine() -> Arc<dyn StateMachine<i32, i32>> {
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
        Some("timed".to_owned()),
        None,
        Vec::new(),
    );

    let transition = TransitionData::with_all(
        1,
        2,
        None,
        None,
        Some(20),
        Some(2),
        Vec::new(),
        None,
        next_web_state_machine::transition::transition_kind::TransitionKind::External,
        None,
        "timer",
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
        Some(TransitionsData::new(vec![transition])),
    );

    BaseStateMachineFactory::<i32, i32>::new(Arc::new(model), None)
        .get_state_machine()
        .expect("the machine builds")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_events_are_serialized() {
    let machine = build_two_state_machine();
    machine.start().await.expect("the machine starts");

    let mut tasks = Vec::new();
    for _ in 0..16 {
        let machine = machine.clone();
        tasks.push(tokio::spawn(async move {
            machine
                .send_event(Arc::new(GenericMessage::with_payload(100)))
                .await
                .expect("the event is sent")
                .result_type()
        }));
    }

    let mut accepted = 0;
    for task in tasks {
        if task.await.expect("the task completes") == ResultType::Accepted {
            accepted += 1;
        }
    }

    assert_eq!(
        accepted, 1,
        "only one of the concurrent events runs the transition"
    );
    assert_eq!(machine.state().map(|state| *state.id()), Some(2));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_start_is_safe() {
    let machine = build_two_state_machine();

    let mut tasks = Vec::new();
    for _ in 0..8 {
        let machine = machine.clone();
        tasks.push(tokio::spawn(async move { machine.start().await }));
    }

    for task in tasks {
        task.await
            .expect("the task completes")
            .expect("the machine starts");
    }

    assert_eq!(
        machine.state().map(|state| *state.id()),
        Some(1),
        "the machine is on its initial state"
    );

    let result = machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");
    assert_eq!(result.result_type(), ResultType::Accepted);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_timer_stops_with_the_machine() {
    let machine = build_timed_machine();
    machine.start().await.expect("the machine starts");

    tokio::time::sleep(Duration::from_millis(120)).await;
    assert_eq!(
        machine.state().map(|state| *state.id()),
        Some(2),
        "the timer moved the machine"
    );

    machine.stop().await.expect("the machine stops");
    assert!(machine.state().is_none(), "a stopped machine has no state");

    // Nothing may happen after the stop, and the machine reports the event as
    // not accepted because it is not running.
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert!(machine.state().is_none());

    let result = machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");
    assert_eq!(result.result_type(), ResultType::Denied);

    // The machine can be started again.
    machine.start().await.expect("the machine starts again");
    assert_eq!(machine.state().map(|state| *state.id()), Some(1));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stopping_while_events_are_in_flight_does_not_deadlock() {
    let machine = build_two_state_machine();
    machine.start().await.expect("the machine starts");

    let mut tasks = Vec::new();
    for _ in 0..4 {
        let machine = machine.clone();
        tasks.push(tokio::spawn(async move {
            for _ in 0..25 {
                let _ = machine
                    .send_event(Arc::new(GenericMessage::with_payload(100)))
                    .await;
            }
        }));
    }

    let stopper = machine.clone();
    tasks.push(tokio::spawn(async move {
        for _ in 0..10 {
            let _ = stopper.stop().await;
            let _ = stopper.start().await;
        }
    }));

    for task in tasks {
        tokio::time::timeout(Duration::from_secs(10), task)
            .await
            .expect("the machine does not deadlock")
            .expect("the task completes");
    }
}
