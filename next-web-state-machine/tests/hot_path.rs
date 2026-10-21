//! Guards the structural properties of the event hot path.
//!
//! The Java implementation hands the same `Message` reference through the whole
//! pipeline and clones one listener collection per dispatch. These tests lock in
//! the Rust equivalent: contexts hand out the same message and listener
//! dispatch shares one snapshot instead of copying the collection.

use std::sync::{Arc, Mutex};

use next_web_core::messaging::generic_message::GenericMessage;
use next_web_core::messaging::Message;
use next_web_state_machine::action::composite_action_listener::CompositeActionListener;
use next_web_state_machine::state::action_listener::ActionListener;
use next_web_state_machine::state::composite_state_listener::CompositeStateListener;
use next_web_state_machine::state::state_listener::StateListener;
use next_web_state_machine::state::object_state::ObjectState;
use next_web_state_machine::state::StateMachineState;
use next_web_state_machine::state_context::{Stage, StateContext};
use next_web_state_machine::support::default_state_context::DefaultStateContext;
use next_web_state_machine::support::default_extended_state::DefaultExtendedState;
use next_web_state_machine::StateMachine as _;
use next_web_state_machine::state_machine_event_result::ResultType;
use next_web_state_machine::support::base_state_machine::BaseStateMachine;
use next_web_state_machine::support::default_state_machine_executor::DefaultStateMachineExecutor;
use next_web_state_machine::transition::base_transition::BaseTransition;
use next_web_state_machine::transition::transition_kind::TransitionKind;
use next_web_state_machine::transition::StateMachineTransition;
use next_web_state_machine::trigger::event_trigger::EventTrigger;
use next_web_state_machine::state::default_pseudo_state::DefaultPseudoState;
use next_web_state_machine::state::pseudo_state_kind::PseudoStateKind;
use next_web_state_machine::region::Region;

type S = u32;
type E = u32;

/// A no-op state listener counting its invocations.
#[derive(Default)]
struct CountingStateListener {
    entries: Mutex<u32>,
}

#[next_web_core::async_trait]
impl StateListener<S, E> for CountingStateListener {
    async fn on_entry(&self, _context: &dyn StateContext<S, E>) {
        *self.entries.lock().expect("lock") += 1;
    }

    async fn on_exit(&self, _context: &dyn StateContext<S, E>) {}

    async fn on_complete(&self, _context: &dyn StateContext<S, E>) {}

    async fn do_on_complete(&self, _context: &dyn StateContext<S, E>) {}
}

/// A no-op action listener.
struct NoopActionListener;

#[next_web_core::async_trait]
impl ActionListener<S, E> for NoopActionListener {
    async fn on_execute(
        &self,
        _state_machine: &dyn next_web_state_machine::StateMachine<S, E>,
        _action: &next_web_state_machine::BoxedStateAction<S, E>,
        _duration: std::time::Duration,
    ) {
    }
}

#[test]
fn a_listener_dispatch_shares_one_snapshot() {
    let composite = CompositeStateListener::<S, E>::new();
    composite.register(Arc::new(CountingStateListener::default()));
    composite.register(Arc::new(CountingStateListener::default()));

    let first = composite.snapshot();
    let second = composite.snapshot();

    // A notification clones the snapshot handle, not the listener vector.
    assert!(Arc::ptr_eq(&first, &second));

    // Registering a listener replaces the snapshot copy-on-write.
    composite.register(Arc::new(CountingStateListener::default()));
    let third = composite.snapshot();
    assert!(!Arc::ptr_eq(&second, &third));
    assert_eq!(third.len(), 3);
}

#[test]
fn an_action_dispatch_shares_one_snapshot() {
    let composite = CompositeActionListener::<S, E>::new();
    composite.register(Arc::new(NoopActionListener));

    let first = composite.snapshot();
    let second = composite.snapshot();
    assert!(Arc::ptr_eq(&first, &second));
}

#[test]
fn a_context_hands_out_the_same_message() {
    let message: Arc<dyn Message<E>> = Arc::new(GenericMessage::with_payload(7u32));
    let context: DefaultStateContext<S, E> = DefaultStateContext::new(
        Stage::Transition,
        Some(message.clone()),
        None,
        Some(Arc::new(DefaultExtendedState::default())),
        None,
        None,
        None,
        None,
        None,
    );

    let handed_out = context.message_arc().expect("context has a message");
    assert!(Arc::ptr_eq(&handed_out, &message));
    assert_eq!(context.event(), Some(&7));
}

/// A machine which can take a couple of events, used to check the result types
/// the pipeline reports.
fn build_machine() -> Arc<BaseStateMachine<S, E>> {
    let initial = Arc::new(ObjectState::<S, E>::new(
        1,
        Some(Arc::new(DefaultPseudoState::<S, E>::new(
            PseudoStateKind::Initial,
        ))),
    ));
    let second = Arc::new(ObjectState::<S, E>::new(2, None));
    let end = Arc::new(ObjectState::<S, E>::new(
        3,
        Some(Arc::new(DefaultPseudoState::<S, E>::new(PseudoStateKind::End))),
    ));

    let states: Vec<Arc<dyn StateMachineState<S, E>>> = vec![
        initial.clone() as Arc<dyn StateMachineState<S, E>>,
        second.clone() as Arc<dyn StateMachineState<S, E>>,
        end.clone() as Arc<dyn StateMachineState<S, E>>,
    ];

    let transitions: Vec<Arc<dyn StateMachineTransition<S, E>>> = vec![
        Arc::new(BaseTransition::new(
            Some(initial.clone() as Arc<dyn StateMachineState<S, E>>),
            second.clone() as Arc<dyn StateMachineState<S, E>>,
            Vec::new(),
            TransitionKind::External,
            None,
            Some(Arc::new(EventTrigger::<S, E>::new(100))),
        )),
        Arc::new(BaseTransition::new(
            Some(second as Arc<dyn StateMachineState<S, E>>),
            end as Arc<dyn StateMachineState<S, E>>,
            Vec::new(),
            TransitionKind::External,
            None,
            Some(Arc::new(EventTrigger::<S, E>::new(200))),
        )),
    ];

    let mut machine = BaseStateMachine::new(
        states,
        transitions,
        initial as Arc<dyn StateMachineState<S, E>>,
        None,
        None,
        Arc::new(DefaultExtendedState::default()),
    );
    machine.init().expect("machine configuration is valid");

    let machine = Arc::new(machine);
    let weak: std::sync::Weak<dyn next_web_state_machine::StateMachine<S, E>> =
        Arc::downgrade(&(machine.clone()
            as Arc<dyn next_web_state_machine::StateMachine<S, E>>));
    let executor = Arc::new(DefaultStateMachineExecutor::new(
        weak,
        None,
        machine.transitions_arc().to_vec(),
        next_web_state_machine::transition::transition_conflict_policy::TransitionConflictPolicy::Child,
    ));
    machine.attach(executor);
    machine
}

#[tokio::test]
async fn a_machine_reaches_its_end_state_and_stops() {    let machine = build_machine();
    machine.start().await.expect("start succeeds");
    assert_eq!(Region::state(machine.as_ref()).map(|state| *state.id()), Some(1));

    let first = machine
        .send_event(Arc::new(GenericMessage::with_payload(100u32)))
        .await
        .expect("event is sent");
    assert_eq!(first.result_type(), ResultType::Accepted);

    let second = machine
        .send_event(Arc::new(GenericMessage::with_payload(200u32)))
        .await
        .expect("event is sent");
    assert_eq!(second.result_type(), ResultType::Accepted);

    // The end state stops the machine.
    assert!(!machine.is_running());
    assert!(machine.is_complete());
}

/// A machine which bounces between two states, used to measure the hot path.
fn build_ping_pong_machine() -> Arc<BaseStateMachine<S, E>> {
    let initial = Arc::new(ObjectState::<S, E>::new(
        1,
        Some(Arc::new(DefaultPseudoState::<S, E>::new(
            PseudoStateKind::Initial,
        ))),
    ));
    let second = Arc::new(ObjectState::<S, E>::new(2, None));

    let states: Vec<Arc<dyn StateMachineState<S, E>>> = vec![
        initial.clone() as Arc<dyn StateMachineState<S, E>>,
        second.clone() as Arc<dyn StateMachineState<S, E>>,
    ];

    let transitions: Vec<Arc<dyn StateMachineTransition<S, E>>> = vec![
        Arc::new(BaseTransition::new(
            Some(initial.clone() as Arc<dyn StateMachineState<S, E>>),
            second.clone() as Arc<dyn StateMachineState<S, E>>,
            Vec::new(),
            TransitionKind::External,
            None,
            Some(Arc::new(EventTrigger::<S, E>::new(100))),
        )),
        Arc::new(BaseTransition::new(
            Some(second as Arc<dyn StateMachineState<S, E>>),
            initial.clone() as Arc<dyn StateMachineState<S, E>>,
            Vec::new(),
            TransitionKind::External,
            None,
            Some(Arc::new(EventTrigger::<S, E>::new(200))),
        )),
    ];

    let mut machine = BaseStateMachine::new(
        states,
        transitions,
        initial as Arc<dyn StateMachineState<S, E>>,
        None,
        None,
        Arc::new(DefaultExtendedState::default()),
    );
    machine.init().expect("machine configuration is valid");
    let machine = Arc::new(machine);
    let weak: std::sync::Weak<dyn next_web_state_machine::StateMachine<S, E>> =
        Arc::downgrade(&(machine.clone()
            as Arc<dyn next_web_state_machine::StateMachine<S, E>>));
    let executor = Arc::new(DefaultStateMachineExecutor::new(
        weak,
        None,
        machine.transitions_arc().to_vec(),
        next_web_state_machine::transition::transition_conflict_policy::TransitionConflictPolicy::Child,
    ));
    machine.attach(executor);
    machine
}

/// Reports the throughput of the event path.
///
/// Run it with
/// `cargo test -p next-web-state-machine --release --test hot_path -- --ignored --nocapture`.
///
/// Baseline measured after the runtime rewrite: three consecutive runs reported
/// 31.21 ms, 31.57 ms and 31.88 ms for 20 000 events, which is a median of
/// 31.57 ms, 1578 ns per event and about 634 000 events per second. A change of
/// more than ten percent is worth investigating.
#[tokio::test]
#[ignore = "reports throughput instead of asserting a threshold"]
async fn throughput_probe() {
    let machine = build_ping_pong_machine();
    machine.start().await.expect("start succeeds");
    machine.set_initial_enabled(false);

    const ROUNDS: usize = 20_000;
    let started = std::time::Instant::now();
    for index in 0..ROUNDS {
        let event = if index % 2 == 0 { 100 } else { 200 };
        let result = machine
            .send_event(Arc::new(GenericMessage::with_payload(event)))
            .await
            .expect("event is sent");
        assert!(result.result_type().is_accepted());
    }
    let elapsed = started.elapsed();

    println!(
        "{} events in {:?} ({:.0} ns/event, {:.0} events/s)",
        ROUNDS,
        elapsed,
        elapsed.as_nanos() as f64 / ROUNDS as f64,
        ROUNDS as f64 / elapsed.as_secs_f64()
    );
}
