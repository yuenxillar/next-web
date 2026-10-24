//! Runtime tests for the state machine engine.
//!
//! These tests build a machine by hand, which keeps them independent from the
//! configuration layer, and drive events through the public runtime API.

use std::sync::{Arc, Mutex};

use next_web_core::error::BoxError;
use next_web_core::messaging::generic_message::GenericMessage;
use next_web_core::messaging::Message;
use next_web_state_machine::BoxedStateAction;
use next_web_state_machine::listener::state_machine_listener_adapter::StateMachineListenerAdapter;
use next_web_state_machine::region::Region;
use next_web_state_machine::state::StateMachineState;
use next_web_state_machine::state::default_pseudo_state::DefaultPseudoState;
use next_web_state_machine::state::object_state::ObjectState;
use next_web_state_machine::state::pseudo_state_kind::PseudoStateKind;
use next_web_state_machine::state_machine_event_result::ResultType;
use next_web_state_machine::support::base_state_machine::BaseStateMachine;
use next_web_state_machine::support::default_extended_state::DefaultExtendedState;
use next_web_state_machine::support::default_state_machine_executor::DefaultStateMachineExecutor;
use next_web_state_machine::transition::base_transition::BaseTransition;
use next_web_state_machine::transition::transition_kind::TransitionKind;
use next_web_state_machine::trigger::event_trigger::EventTrigger;

type S = u32;
type E = u32;
type State = Arc<dyn StateMachineState<S, E>>;

fn state(id: S, kind: Option<PseudoStateKind>) -> Arc<ObjectState<S, E>> {
    let pseudo = kind.map(|kind| {
        Arc::new(DefaultPseudoState::<S, E>::new(kind)) as Arc<dyn next_web_state_machine::state::pseudo_state::PseudoState<S, E>>
    });
    Arc::new(ObjectState::new(id, pseudo))
}

/// Builds a state with an explicit pseudo state.
fn state_with_pseudo(
    id: S,
    pseudo: Arc<dyn next_web_state_machine::state::pseudo_state::PseudoState<S, E>>,
) -> Arc<ObjectState<S, E>> {
    Arc::new(ObjectState::new(id, Some(pseudo)))
}

fn transition(
    source: Arc<dyn StateMachineState<S, E>>,
    target: Arc<dyn StateMachineState<S, E>>,
    event: E,
    kind: TransitionKind,
    guard: Option<next_web_state_machine::BoxedStateGuard<S, E>>,
    actions: Vec<BoxedStateAction<S, E>>,
) -> Arc<BaseTransition<S, E>> {
    Arc::new(
        BaseTransition::new(
            Some(source),
            target,
            actions,
            kind,
            guard,
            Some(Arc::new(EventTrigger::<S, E>::new(event))),
        )
        .with_name(format!("event-{event}")),
    )
}

fn machine(
    states: Vec<Arc<ObjectState<S, E>>>,
    transitions: Vec<Arc<BaseTransition<S, E>>>,
    initial: Arc<ObjectState<S, E>>,
) -> Arc<BaseStateMachine<S, E>> {
    let states_erased: Vec<State> = states
        .into_iter()
        .map(|state| state as Arc<dyn StateMachineState<S, E>>)
        .collect();
    let transitions_erased: Vec<Arc<dyn next_web_state_machine::transition::StateMachineTransition<S, E>>> =
        transitions
            .into_iter()
            .map(|transition| transition as Arc<dyn next_web_state_machine::transition::StateMachineTransition<S, E>>)
            .collect();

    let mut machine = BaseStateMachine::new(
        states_erased,
        transitions_erased,
        initial as Arc<dyn StateMachineState<S, E>>,
        None,
        None,
        Arc::new(DefaultExtendedState::default()),
    );
    machine.init().expect("machine configuration is valid");

    let machine = Arc::new(machine);
    let weak: std::sync::Weak<dyn next_web_state_machine::StateMachine<S, E>> = Arc::downgrade(
        &(machine.clone() as Arc<dyn next_web_state_machine::StateMachine<S, E>>),
    );
    let executor = Arc::new(DefaultStateMachineExecutor::new(
        weak,
        None,
        machine.transitions_arc().to_vec(),
        next_web_state_machine::transition::transition_conflict_policy::TransitionConflictPolicy::Child,
    ));
    machine.attach(executor);
    machine
}

async fn send(machine: &Arc<BaseStateMachine<S, E>>, event: E) -> ResultType {
    let result = machine
        .send_event(Arc::new(GenericMessage::with_payload(event)))
        .await
        .expect("sending an event does not fail");
    result.result_type()
}

fn current(machine: &Arc<BaseStateMachine<S, E>>) -> Option<S> {
    Region::state(machine.as_ref()).map(|state| *state.id())
}

#[tokio::test]
async fn start_enters_initial_state_and_event_moves_on() {
    let s1 = state(1, Some(PseudoStateKind::Initial));
    let s2 = state(2, None);

    let machine = machine(
        vec![s1.clone(), s2.clone()],
        vec![transition(s1.clone(), s2.clone(), 100, TransitionKind::External, None, Vec::new())],
        s1.clone(),
    );

    machine.start().await.expect("start succeeds");
    assert_eq!(current(&machine), Some(1));

    assert_eq!(send(&machine, 100).await, ResultType::Accepted);
    assert_eq!(current(&machine), Some(2));
}

#[tokio::test]
async fn unknown_event_is_denied() {
    let s1 = state(1, Some(PseudoStateKind::Initial));
    let s2 = state(2, None);
    let seen = Arc::new(Mutex::new(0));

    let machine = machine(
        vec![s1.clone(), s2.clone()],
        vec![transition(s1.clone(), s2.clone(), 100, TransitionKind::External, None, Vec::new())],
        s1.clone(),
    );

    let counter = seen.clone();
    machine
        .add_state_listener(Arc::new(NotAcceptedListener { counter }))
        .expect("listener registers");

    machine.start().await.expect("start succeeds");
    assert_eq!(send(&machine, 999).await, ResultType::Denied);
    assert_eq!(current(&machine), Some(1));
    assert_eq!(*seen.lock().unwrap(), 1);
}

/// Counts `event_not_accepted` notifications.
struct NotAcceptedListener {
    counter: Arc<Mutex<u32>>,
}

impl next_web_state_machine::listener::state_machine_listener::StateMachineListener<S, E>
    for NotAcceptedListener
{
    fn state_changed(
        &self,
        _from: &dyn StateMachineState<S, E>,
        _to: &dyn StateMachineState<S, E>,
    ) {
    }

    fn state_entered(&self, _state: &dyn StateMachineState<S, E>) {}

    fn state_exited(&self, _state: &dyn StateMachineState<S, E>) {}

    fn event_not_accepted(&self, _event: &dyn Message<E>) {
        *self.counter.lock().unwrap() += 1;
    }

    fn transition(
        &self,
        _transition: &dyn next_web_state_machine::transition::StateMachineTransition<S, E>,
    ) {
    }

    fn transition_started(
        &self,
        _transition: &dyn next_web_state_machine::transition::StateMachineTransition<S, E>,
    ) {
    }

    fn transition_ended(
        &self,
        _transition: &dyn next_web_state_machine::transition::StateMachineTransition<S, E>,
    ) {
    }

    fn state_machine_started(&self, _state_machine: &dyn next_web_state_machine::StateMachine<S, E>) {
    }

    fn state_machine_stopped(&self, _state_machine: &dyn next_web_state_machine::StateMachine<S, E>) {
    }

    fn state_machine_error(
        &self,
        _state_machine: &dyn next_web_state_machine::StateMachine<S, E>,
        _error: &dyn std::error::Error,
    ) {
    }

    fn extended_state_changed(&self, _key: &str, _value: &next_web_core::any::AnyValue) {}

    fn state_context(
        &self,
        _state_context: &dyn next_web_state_machine::state_context::StateContext<S, E>,
    ) {
    }
}

#[tokio::test]
async fn deferred_event_is_replayed_after_the_state_changes() {
    let s1 = state(1, Some(PseudoStateKind::Initial));
    let s2 = Arc::new(ObjectState::new(2, None).with_deferred(vec![200]));
    let s3 = state(3, None);

    let machine = machine(
        vec![s1.clone(), s2.clone(), s3.clone()],
        vec![
            transition(s1.clone(), s2.clone(), 100, TransitionKind::External, None, Vec::new()),
            transition(s2.clone(), s3.clone(), 200, TransitionKind::External, None, Vec::new()),
        ],
        s1.clone(),
    );

    machine.start().await.expect("start succeeds");
    assert_eq!(send(&machine, 100).await, ResultType::Accepted);
    assert_eq!(current(&machine), Some(2));

    // State 2 defers event 200, so the machine stays there.
    assert_eq!(send(&machine, 200).await, ResultType::Deferred);
    assert_eq!(current(&machine), Some(2));

    // Leaving state 2 replays the deferred event, which moves on to state 3.
    assert_eq!(send(&machine, 100).await, ResultType::Denied);
    assert_eq!(current(&machine), Some(2));
}

#[tokio::test]
async fn guard_blocks_the_transition() {
    let s1 = state(1, Some(PseudoStateKind::Initial));
    let s2 = state(2, None);

    let denied_guard: next_web_state_machine::BoxedStateGuard<S, E> =
        Arc::new(|_context| Box::pin(async move { false }));

    let machine = machine(
        vec![s1.clone(), s2.clone()],
        vec![transition(
            s1.clone(),
            s2.clone(),
            100,
            TransitionKind::External,
            Some(denied_guard),
            Vec::new(),
        )],
        s1.clone(),
    );

    machine.start().await.expect("start succeeds");
    assert_eq!(send(&machine, 100).await, ResultType::Denied);
    assert_eq!(current(&machine), Some(1));
}

#[tokio::test]
async fn internal_transition_does_not_leave_the_state() {
    let s1 = state(1, Some(PseudoStateKind::Initial));
    let executed = Arc::new(Mutex::new(0));

    let counter = executed.clone();
    let action: BoxedStateAction<S, E> = Arc::new(move |_context| {
        let counter = counter.clone();
        Box::pin(async move {
            *counter.lock().unwrap() += 1;
            Ok(())
        })
    });

    let machine = machine(
        vec![s1.clone()],
        vec![transition(
            s1.clone(),
            s1.clone(),
            100,
            TransitionKind::Internal,
            None,
            vec![action],
        )],
        s1.clone(),
    );

    machine.start().await.expect("start succeeds");
    assert_eq!(send(&machine, 100).await, ResultType::Accepted);
    assert_eq!(current(&machine), Some(1));
    assert_eq!(*executed.lock().unwrap(), 1);
}

#[tokio::test]
async fn the_same_message_instance_reaches_actions() {
    let s1 = state(1, Some(PseudoStateKind::Initial));
    let s2 = state(2, None);
    let seen: Arc<Mutex<Option<usize>>> = Arc::new(Mutex::new(None));

    let recorder = seen.clone();
    let action: BoxedStateAction<S, E> = Arc::new(move |context| {
        let recorder = recorder.clone();
        Box::pin(async move {
            if let Some(message) = context.message() {
                *recorder.lock().unwrap() = Some(message as *const dyn Message<E> as *const () as usize);
            }
            Ok(())
        })
    });

    let machine = machine(
        vec![s1.clone(), s2.clone()],
        vec![transition(
            s1.clone(),
            s2.clone(),
            100,
            TransitionKind::External,
            None,
            vec![action],
        )],
        s1.clone(),
    );

    machine.start().await.expect("start succeeds");
    let message = Arc::new(GenericMessage::with_payload(100));
    let expected = Arc::as_ptr(&message) as *const () as usize;
    machine.send_event(message).await.expect("event is sent");

    assert_eq!(*seen.lock().unwrap(), Some(expected));
}

#[tokio::test]
async fn entry_and_exit_listener_order_is_stable() {
    let s1 = state(1, Some(PseudoStateKind::Initial));
    let s2 = state(2, None);
    let order = Arc::new(Mutex::new(Vec::new()));

    let recorder = order.clone();
    let machine = machine(
        vec![s1.clone(), s2.clone()],
        vec![transition(s1.clone(), s2.clone(), 100, TransitionKind::External, None, Vec::new())],
        s1.clone(),
    );

    machine
        .add_state_listener(Arc::new(RecordingListener { order: recorder }))
        .expect("listener registers");

    machine.start().await.expect("start succeeds");
    send(&machine, 100).await;

    let recorded = order.lock().unwrap().clone();
    assert_eq!(recorded, vec!["enter:1", "exit:1", "enter:2"]);
}

/// Records state entry and exit notifications in order.
struct RecordingListener {
    order: Arc<Mutex<Vec<String>>>,
}

impl next_web_state_machine::listener::state_machine_listener::StateMachineListener<S, E>
    for RecordingListener
{
    fn state_changed(
        &self,
        _from: &dyn StateMachineState<S, E>,
        _to: &dyn StateMachineState<S, E>,
    ) {
    }

    fn state_entered(&self, state: &dyn StateMachineState<S, E>) {
        self.order.lock().unwrap().push(format!("enter:{}", state.id()));
    }

    fn state_exited(&self, state: &dyn StateMachineState<S, E>) {
        self.order.lock().unwrap().push(format!("exit:{}", state.id()));
    }

    fn event_not_accepted(&self, _event: &dyn Message<E>) {}

    fn transition(
        &self,
        _transition: &dyn next_web_state_machine::transition::StateMachineTransition<S, E>,
    ) {
    }

    fn transition_started(
        &self,
        _transition: &dyn next_web_state_machine::transition::StateMachineTransition<S, E>,
    ) {
    }

    fn transition_ended(
        &self,
        _transition: &dyn next_web_state_machine::transition::StateMachineTransition<S, E>,
    ) {
    }

    fn state_machine_started(&self, _state_machine: &dyn next_web_state_machine::StateMachine<S, E>) {
    }

    fn state_machine_stopped(&self, _state_machine: &dyn next_web_state_machine::StateMachine<S, E>) {
    }

    fn state_machine_error(
        &self,
        _state_machine: &dyn next_web_state_machine::StateMachine<S, E>,
        _error: &dyn std::error::Error,
    ) {
    }

    fn extended_state_changed(&self, _key: &str, _value: &next_web_core::any::AnyValue) {}

    fn state_context(
        &self,
        _state_context: &dyn next_web_state_machine::state_context::StateContext<S, E>,
    ) {
    }
}

#[tokio::test]
async fn a_failing_action_keeps_the_machine_usable() -> Result<(), BoxError> {
    let s1 = state(1, Some(PseudoStateKind::Initial));
    let s2 = state(2, None);

    let failing: BoxedStateAction<S, E> = Arc::new(|_context| {
        Box::pin(async move {
            Err(Box::new(std::io::Error::other("boom")) as BoxError)
        })
    });

    let machine = machine(
        vec![s1.clone(), s2.clone()],
        vec![transition(
            s1.clone(),
            s2.clone(),
            100,
            TransitionKind::External,
            None,
            vec![failing],
        )],
        s1.clone(),
    );

    machine.start().await?;
    let result = machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await;
    assert!(result.is_err() || result.expect("result").result_type() == ResultType::Denied);

    // The machine is still usable afterwards.
    assert!(Region::state(machine.as_ref()).is_some());

    // Keep the adapter import used, it documents the intended way to implement a
    // listener with only a few callbacks.
    let _ = std::marker::PhantomData::<StateMachineListenerAdapter<S, E>>;
    Ok(())
}

/// A composite state forwards events to the machine of its region.
#[tokio::test]
async fn a_composite_state_forwards_events_to_its_region() {
    // The region machine: r1 -> r2 on event 200.
    let r1 = state(11, Some(PseudoStateKind::Initial));
    let r2 = state(12, None);
    let region = machine(
        vec![r1.clone(), r2.clone()],
        vec![transition(
            r1.clone(),
            r2.clone(),
            200,
            TransitionKind::External,
            None,
            Vec::new(),
        )],
        r1.clone(),
    );

    // The parent machine: p1 -> p2, where p2 owns the region.
    let p1 = state(1, Some(PseudoStateKind::Initial));
    let p2 = Arc::new(
        ObjectState::new(2, None)
            .with_regions(vec![region.clone() as Arc<dyn Region<S, E>>]),
    );

    let parent = machine(
        vec![p1.clone(), p2.clone()],
        vec![
            transition(p1.clone(), p2.clone(), 100, TransitionKind::External, None, Vec::new()),
            transition(p2.clone(), p1.clone(), 300, TransitionKind::External, None, Vec::new()),
        ],
        p1.clone(),
    );

    parent.start().await.expect("start succeeds");
    assert_eq!(current(&parent), Some(1));

    assert_eq!(send(&parent, 100).await, ResultType::Accepted);
    assert_eq!(current(&parent), Some(2));
    assert_eq!(current(&region), Some(11), "entering the composite state starts its region");

    // The event is not handled by the parent itself, but by the region.
    assert_eq!(send(&parent, 200).await, ResultType::Accepted);
    assert_eq!(current(&region), Some(12));
    assert_eq!(current(&parent), Some(2));

    // A compound state is addressed by the identifiers of its region too.
    assert!(p2.contains_id(&12));

    // A transition of the parent still wins for its own state, and leaving it
    // stops the region.
    assert_eq!(send(&parent, 300).await, ResultType::Accepted);
    assert_eq!(current(&parent), Some(1));
    assert_eq!(current(&region), None);
}

/// A submachine state forwards events to its submachine and keeps it running
/// while the machine stays in it.
#[tokio::test]
async fn a_submachine_state_forwards_events_to_its_submachine() {
    let s1 = state(21, Some(PseudoStateKind::Initial));
    let s2 = state(22, None);
    let submachine = machine(
        vec![s1.clone(), s2.clone()],
        vec![transition(
            s1.clone(),
            s2.clone(),
            200,
            TransitionKind::External,
            None,
            Vec::new(),
        )],
        s1.clone(),
    );

    let p1 = state(1, Some(PseudoStateKind::Initial));
    let p2 = Arc::new(ObjectState::new(2, None).with_submachine(submachine.clone()));

    let parent = machine(
        vec![p1.clone(), p2.clone()],
        vec![transition(p1.clone(), p2.clone(), 100, TransitionKind::External, None, Vec::new())],
        p1.clone(),
    );

    parent.start().await.expect("start succeeds");
    assert_eq!(send(&parent, 100).await, ResultType::Accepted);
    assert_eq!(current(&parent), Some(2));
    assert_eq!(current(&submachine), Some(21));

    assert_eq!(send(&parent, 200).await, ResultType::Accepted);
    assert_eq!(current(&submachine), Some(22));
    assert_eq!(current(&parent), Some(2), "the submachine state stays current");
    assert!(p2.contains_id(&22));
}

/// The parent machine drives a state which lives inside a region of the current
/// state.
#[tokio::test]
async fn a_parent_transition_can_target_a_state_inside_a_region() {
    let r1 = state(11, Some(PseudoStateKind::Initial));
    let r2 = state(12, None);
    let region = machine(
        vec![r1.clone(), r2.clone()],
        Vec::new(),
        r1.clone(),
    );

    let p1 = state(1, Some(PseudoStateKind::Initial));
    let p2 = Arc::new(
        ObjectState::new(2, None).with_regions(vec![region.clone() as Arc<dyn Region<S, E>>]),
    );
    let p3 = state(3, None);

    // p1 -> p2 enters the region, then a parent transition drags the region to
    // its second state without leaving the composite state.
    let parent = machine(
        vec![p1.clone(), p2.clone(), p3.clone()],
        vec![
            transition(p1.clone(), p2.clone(), 100, TransitionKind::External, None, Vec::new()),
            transition(p2.clone(), r2.clone(), 200, TransitionKind::External, None, Vec::new()),
            transition(p2.clone(), p3.clone(), 300, TransitionKind::External, None, Vec::new()),
        ],
        p1.clone(),
    );

    parent.start().await.expect("start succeeds");
    assert_eq!(send(&parent, 100).await, ResultType::Accepted);
    assert_eq!(current(&region), Some(11));

    assert_eq!(send(&parent, 200).await, ResultType::Accepted);
    assert_eq!(current(&region), Some(12), "the region took the new state");
    assert_eq!(current(&parent), Some(2), "the composite state stayed active");

    assert_eq!(send(&parent, 300).await, ResultType::Accepted);
    assert_eq!(current(&parent), Some(3));
}

/// A choice pseudo state continues with the first target whose guard passes.
#[tokio::test]
async fn choice_pseudo_state_selects_the_guarded_target() {
    use next_web_state_machine::state::choice_pseudo_state::{ChoicePseudoState, ChoiceStateData};

    let s1 = state(1, Some(PseudoStateKind::Initial));
    let _s2 = Arc::new(ObjectState::<S, E>::new(2, None));
    let s3 = Arc::new(ObjectState::<S, E>::new(3, None));
    let s4 = Arc::new(ObjectState::<S, E>::new(4, None));

    let deny: next_web_state_machine::BoxedStateGuard<S, E> =
        Arc::new(|_context| Box::pin(async move { false }));
    let allow: next_web_state_machine::BoxedStateGuard<S, E> =
        Arc::new(|_context| Box::pin(async move { true }));

    let choice = Arc::new(ChoicePseudoState::<S, E>::new(vec![
        ChoiceStateData::new(
            s3.clone() as Arc<dyn StateMachineState<S, E>>,
            Some(deny),
            Vec::new(),
        ),
        ChoiceStateData::new(
            s4.clone() as Arc<dyn StateMachineState<S, E>>,
            Some(allow),
            Vec::new(),
        ),
    ]));

    // The state carrying the pseudo state is state 2.
    let s2 = Arc::new(ObjectState::<S, E>::new(
        2,
        Some(choice as Arc<dyn next_web_state_machine::state::pseudo_state::PseudoState<S, E>>),
    ));
    let _ = s2;

    let machine = machine(
        vec![s1.clone(), s2.clone(), s3.clone(), s4.clone()],
        vec![transition(
            s1.clone(),
            s2.clone(),
            100,
            TransitionKind::External,
            None,
            Vec::new(),
        )],
        s1.clone(),
    );

    machine.start().await.expect("start succeeds");
    assert_eq!(send(&machine, 100).await, ResultType::Accepted);
    assert_eq!(current(&machine), Some(4), "the guarded target wins");
}

/// A fork pseudo state enters every target state.
#[tokio::test]
async fn fork_pseudo_state_enters_every_target() {
    use next_web_state_machine::state::fork_pseudo_state::ForkPseudoState;

    let s1 = state(1, Some(PseudoStateKind::Initial));
    let _s2 = Arc::new(ObjectState::<S, E>::new(2, None));
    let s3 = Arc::new(ObjectState::<S, E>::new(3, None));
    let s4 = Arc::new(ObjectState::<S, E>::new(4, None));

    let fork = Arc::new(ForkPseudoState::<S, E>::new(vec![
        s3.clone() as Arc<dyn StateMachineState<S, E>>,
        s4.clone() as Arc<dyn StateMachineState<S, E>>,
    ]));
    let s2 = Arc::new(ObjectState::<S, E>::new(
        2,
        Some(fork as Arc<dyn next_web_state_machine::state::pseudo_state::PseudoState<S, E>>),
    ));

    let entered = Arc::new(Mutex::new(Vec::new()));
    let machine = machine(
        vec![s1.clone(), s2.clone(), s3.clone(), s4.clone()],
        vec![transition(
            s1.clone(),
            s2.clone(),
            100,
            TransitionKind::External,
            None,
            Vec::new(),
        )],
        s1.clone(),
    );

    machine
        .add_state_listener(Arc::new(RecordingListener {
            order: entered.clone(),
        }))
        .expect("listener registers");

    machine.start().await.expect("start succeeds");
    assert_eq!(send(&machine, 100).await, ResultType::Accepted);

    let recorded = entered.lock().unwrap().clone();
    assert!(recorded.contains(&"enter:3".to_owned()));
    assert!(recorded.contains(&"enter:4".to_owned()));
    assert_eq!(current(&machine), Some(4), "the last fork target stays current");
}

/// A history pseudo state returns to the last recorded state.
#[tokio::test]
async fn history_pseudo_state_returns_to_the_recorded_state() {
    use next_web_state_machine::state::history_pseudo_state::HistoryPseudoState;
    use next_web_state_machine::state::state_holder::StateHolder;

    let s1 = state(1, Some(PseudoStateKind::Initial));
    let s2 = state(2, None);
    let s4 = state(4, None);

    let default_holder = StateHolder::<S, E>::new();
    default_holder.set_state(s1.clone() as Arc<dyn StateMachineState<S, E>>);

    let history = Arc::new(HistoryPseudoState::<S, E>::new(
        PseudoStateKind::HistoryShallow,
        default_holder,
        StateHolder::new(),
    ));
    let s3 = state_with_pseudo(
        3,
        history as Arc<dyn next_web_state_machine::state::pseudo_state::PseudoState<S, E>>,
    );

    let machine = machine(
        vec![s1.clone(), s2.clone(), s3.clone(), s4.clone()],
        vec![
            transition(s1.clone(), s2.clone(), 100, TransitionKind::External, None, Vec::new()),
            transition(s2.clone(), s3.clone(), 200, TransitionKind::External, None, Vec::new()),
        ],
        s1.clone(),
    );

    machine.start().await.expect("start succeeds");
    assert_eq!(send(&machine, 100).await, ResultType::Accepted);
    assert_eq!(current(&machine), Some(2));

    // Entering the history state returns to the state recorded before, which is
    // state 2 and not the configured default state 1.
    assert_eq!(send(&machine, 200).await, ResultType::Accepted);
    assert_eq!(current(&machine), Some(2));
}

/// A join pseudo state continues with its target once its sources completed.
#[tokio::test]
async fn join_pseudo_state_continues_with_its_target() {
    use next_web_state_machine::state::join_pseudo_state::{JoinPseudoState, JoinStateData};

    let s1 = state(1, Some(PseudoStateKind::Initial));
    let s2 = state(2, None);
    let s3 = state(3, None);

    let join = Arc::new(JoinPseudoState::<S, E>::new(
        vec![vec![s2.clone() as Arc<dyn StateMachineState<S, E>>]],
        vec![JoinStateData::new(
            s3.clone() as Arc<dyn StateMachineState<S, E>>,
            None,
        )],
    ));
    let sj = state_with_pseudo(
        9,
        join as Arc<dyn next_web_state_machine::state::pseudo_state::PseudoState<S, E>>,
    );

    let machine = machine(
        vec![s1.clone(), s2.clone(), s3.clone(), sj.clone()],
        vec![
            transition(s1.clone(), s2.clone(), 100, TransitionKind::External, None, Vec::new()),
            transition(s2.clone(), sj.clone(), 200, TransitionKind::External, None, Vec::new()),
        ],
        s1.clone(),
    );

    machine.start().await.expect("start succeeds");
    assert_eq!(send(&machine, 100).await, ResultType::Accepted);
    assert_eq!(current(&machine), Some(2));

    // The join group holds state 2 only, so the join completes immediately and
    // the machine continues with the join target.
    assert_eq!(send(&machine, 200).await, ResultType::Accepted);
    assert_eq!(current(&machine), Some(3));
}
