//! Tests of the semantics which the first round of tests did not cover:
//! deferred events under concurrency, three levels of machines, a transition
//! which is scoped to the state which owns it, and a machine which declares
//! orthogonal regions through its model.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use next_web_core::error::BoxError;
use next_web_core::messaging::generic_message::GenericMessage;
use next_web_state_machine::BoxedStateAction;
use next_web_state_machine::StateMachine;
use next_web_state_machine::config::base_state_machine_factory::BaseStateMachineFactory;
use next_web_state_machine::config::model::configuration_data::ConfigurationData;
use next_web_state_machine::config::model::default_state_machine_model::DefaultStateMachineModel;
use next_web_state_machine::config::model::state_data::StateData;
use next_web_state_machine::config::model::transition_data::TransitionData;
use next_web_state_machine::config::model::transitions_data::TransitionsData;
use next_web_state_machine::config::state_machine_factory::StateMachineFactory;
use next_web_state_machine::state_machine_event_result::ResultType;
use next_web_state_machine::state_context::StateContext;

/// Builds a configuration with the defaults.
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

/// Builds a machine from a state and a transition list.
fn build(
    machine_id: &str,
    states: Vec<StateData<i32, i32>>,
    transitions: Vec<TransitionData<i32, i32>>,
) -> Result<Arc<dyn StateMachine<i32, i32>>, BoxError> {
    let model = DefaultStateMachineModel::new(
        configuration(machine_id),
        Some(states.into()),
        Some(TransitionsData::new(transitions)),
    );

    BaseStateMachineFactory::<i32, i32>::new(Arc::new(model), None).get_state_machine()
}

/// An action which counts how often it ran.
fn counting_action(counter: Arc<AtomicUsize>) -> BoxedStateAction<i32, i32> {
    Arc::new(move |_context: &dyn StateContext<i32, i32>| {
        let counter = counter.clone();

        Box::pin(async move {
            counter.fetch_add(1, Ordering::SeqCst);

            Ok(())
        })
    })
}

/// A deferred event which two tasks send at the same time is postponed by the
/// state which defers it and replayed when that state is left, exactly once.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_deferred_event_sent_concurrently_is_replayed_once() {
    let counter = Arc::new(AtomicUsize::new(0));

    let mut initial = StateData::<i32, i32>::with_initial(1, true);
    initial.set_deferred(vec![200]);
    initial.set_entry_actions(vec![counting_action(counter.clone())]);

    let machine = build(
        "deferred",
        vec![initial, StateData::<i32, i32>::new(2)],
        vec![TransitionData::new(1, 2, 100)],
    )
    .expect("the machine builds");

    machine.start().await.expect("the machine starts");

    // The event 200 is deferred while the machine is in state 1, and the event
    // 100 leaves the state, which replays the deferred one.
    let deferred = machine
        .send_event(Arc::new(GenericMessage::with_payload(200)))
        .await
        .expect("the deferred event is sent");

    assert_eq!(deferred.result_type(), ResultType::Deferred);

    let leaving = machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");

    assert_eq!(leaving.result_type(), ResultType::Accepted);
    assert_eq!(machine.state().map(|state| *state.id()), Some(2));

    // The replayed event must not loop: the machine stays in state 2 and the
    // entry of state 1 ran once.
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

/// A machine three levels deep forwards an event down to the innermost one.
#[tokio::test]
async fn three_levels_of_machines_forward_an_event_to_the_innermost_one() {
    let inner = build(
        "inner",
        vec![
            StateData::<i32, i32>::with_initial(31, true),
            StateData::<i32, i32>::new(32),
        ],
        vec![TransitionData::new(31, 32, 300)],
    )
    .expect("the innermost machine builds");

    let mut middle_state_22 = StateData::<i32, i32>::new(22);
    middle_state_22.set_submachine(inner.clone());

    let middle = build(
        "middle",
        vec![
            StateData::<i32, i32>::with_initial(21, true),
            middle_state_22,
        ],
        vec![TransitionData::new(21, 22, 200)],
    )
    .expect("the middle machine builds");

    let mut outer_state_2 = StateData::<i32, i32>::new(2);
    outer_state_2.set_submachine(middle.clone());

    let outer = build(
        "outer",
        vec![StateData::<i32, i32>::with_initial(1, true), outer_state_2],
        vec![TransitionData::new(1, 2, 100)],
    )
    .expect("the outer machine builds");

    outer.start().await.expect("the outer machine starts");

    for event in [100, 200] {
        let result = outer
            .send_event(Arc::new(GenericMessage::with_payload(event)))
            .await
            .expect("the event is sent");

        assert_eq!(
            result.result_type(),
            ResultType::Accepted,
            "the event {event} is accepted on the way down"
        );
    }

    assert_eq!(outer.state().map(|state| *state.id()), Some(2));
    assert_eq!(middle.state().map(|state| *state.id()), Some(22));
    assert_eq!(
        inner.state().map(|state| *state.id()),
        Some(31),
        "the innermost machine is entered by the middle one"
    );

    let result = outer
        .send_event(Arc::new(GenericMessage::with_payload(300)))
        .await
        .expect("the event is sent");

    assert_eq!(
        result.result_type(),
        ResultType::Accepted,
        "the event reached the innermost machine through two levels"
    );
    assert_eq!(inner.state().map(|state| *state.id()), Some(32));
}

/// A transition which a state declares for the machine it owns runs once, in
/// that machine, and is not registered on the machine which owns the state.
#[tokio::test]
async fn a_scoped_transition_runs_once() {
    let counter = Arc::new(AtomicUsize::new(0));

    // The state 2 owns the states 11 and 12, and the transition 11 -> 12 is
    // declared with the state 2 as its owner.
    let mut state_11 = StateData::<i32, i32>::with_initial(11, true);
    state_11.set_parent(Arc::new(2));

    let mut state_12 = StateData::<i32, i32>::new(12);
    state_12.set_parent(Arc::new(2));
    state_12.set_entry_actions(vec![counting_action(counter.clone())]);

    let scoped = TransitionData::with_all(
        11,
        12,
        Some(2),
        Some(400),
        None,
        None,
        Vec::new(),
        None,
        next_web_state_machine::transition::transition_kind::TransitionKind::External,
        None,
        "scoped",
    );

    let machine = build(
        "scoped",
        vec![
            StateData::<i32, i32>::with_initial(1, true),
            StateData::<i32, i32>::new(2),
            state_11,
            state_12,
        ],
        vec![TransitionData::new(1, 2, 100), scoped],
    )
    .expect("the machine builds");

    // The transition of the child belongs to the child, not to the machine which
    // owns the child.
    assert_eq!(
        machine.transitions().len(),
        1,
        "the machine only holds the transition it declares for itself"
    );

    machine.start().await.expect("the machine starts");

    machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");

    assert_eq!(machine.state().map(|state| *state.id()), Some(2));

    let result = machine
        .send_event(Arc::new(GenericMessage::with_payload(400)))
        .await
        .expect("the event is sent");

    assert_eq!(
        result.result_type(),
        ResultType::Accepted,
        "the machine forwards the event to the child which owns the transition"
    );
    assert_eq!(
        counter.load(Ordering::SeqCst),
        1,
        "the scoped transition ran once, so it is registered once"
    );
}

/// A machine which declares its orthogonal regions through the model builds the
/// two regions and moves both of them.
#[tokio::test]
async fn orthogonal_regions_declared_through_the_model_move_independently() {
    // The state 2 owns the regions "a" and "b"; each region has its own initial
    // state and its own transition.
    let mut region_a = StateData::<i32, i32>::with_initial(11, true);
    region_a.set_parent(Arc::new(2));
    region_a.set_region("a");

    let mut region_a_target = StateData::<i32, i32>::new(12);
    region_a_target.set_parent(Arc::new(2));
    region_a_target.set_region("a");

    let mut region_b = StateData::<i32, i32>::with_initial(21, true);
    region_b.set_parent(Arc::new(2));
    region_b.set_region("b");

    let mut region_b_target = StateData::<i32, i32>::new(22);
    region_b_target.set_parent(Arc::new(2));
    region_b_target.set_region("b");

    let states = vec![
        StateData::<i32, i32>::with_initial(1, true),
        StateData::<i32, i32>::new(2),
        region_a,
        region_a_target,
        region_b,
        region_b_target,
    ];

    let transitions = vec![
        TransitionData::new(1, 2, 100),
        TransitionData::new(11, 12, 200),
        TransitionData::new(21, 22, 300),
    ];

    let result = build("regions", states, transitions);

    match result {
        Ok(machine) => {
            machine.start().await.expect("the machine starts");

            machine
                .send_event(Arc::new(GenericMessage::with_payload(100)))
                .await
                .expect("the event is sent");

            assert_eq!(machine.state().map(|state| *state.id()), Some(2));
        }
        Err(error) => panic!("a model with orthogonal regions did not build: {error}"),
    }
}
