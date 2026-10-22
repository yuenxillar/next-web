# next-web-state-machine

A state machine for Next Web applications: states and transitions, orthogonal
regions, timers, guards and actions, a service layer which owns a machine, and
persistence of the current state and of the extended state.

## Quick start

```toml
[dependencies]
next-web = { version = "0.2", features = ["enable-state-machine"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
use std::sync::Arc;

use next_web::core::messaging::generic_message::GenericMessage;
use next_web::state_machine::config::base_state_machine_factory::BaseStateMachineFactory;
use next_web::state_machine::config::model::configuration_data::ConfigurationData;
use next_web::state_machine::config::model::default_state_machine_model::DefaultStateMachineModel;
use next_web::state_machine::config::model::state_data::StateData;
use next_web::state_machine::config::model::transition_data::TransitionData;
use next_web::state_machine::config::model::transitions_data::TransitionsData;
use next_web::state_machine::config::state_machine_factory::StateMachineFactory;
use next_web::state_machine::region::Region;

#[tokio::main]
async fn main() {
    let configuration = ConfigurationData::<i32, i32>::with_parameters(
        false, None, Vec::new(), false, None, None, None, None, false, None,
        Some("basic".to_owned()), None, Vec::new(),
    );

    let model = DefaultStateMachineModel::new(
        configuration,
        Some(vec![
            StateData::<i32, i32>::with_initial(1, true),
            StateData::<i32, i32>::new(2),
            StateData::<i32, i32>::new(3),
        ].into()),
        Some(TransitionsData::new(vec![
            TransitionData::new(1, 2, 100),
            TransitionData::new(2, 3, 200),
        ])),
    );

    let machine = BaseStateMachineFactory::<i32, i32>::new(Arc::new(model), None)
        .get_state_machine()
        .expect("the machine builds");

    machine.start().await.expect("the machine starts");
    machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");
    machine
        .send_event(Arc::new(GenericMessage::with_payload(200)))
        .await
        .expect("the event is sent");

    println!("state: {}", machine.state().map(|state| *state.id()).expect("a state"));
}
```

The same machine is `examples/state_machine_basic.rs` in this repository, which
also asserts every step and stops the machine.

## What it supports

- States and transitions, including internal, external and local transitions.
- Guards, entry actions, exit actions and state actions.
- Composite states: a state which owns a submachine forwards events to it.
- Orthogonal regions, fork and join.
- Deferred events, which a state postpones until it is left.
- History pseudo states.
- Timed transitions, which start counting when the source state of the
  transition is entered. The platform timer bounds the resolution: on Windows a
  period below about 16ms is rounded up, so measure before you rely on one.
- Extended state variables with change notifications.
- Listeners, interceptors and monitors for observability.
- Security rules for events and transitions, enforced fail-closed.
- Persistence of the current state and of the extended state variables.
- A service layer which builds, starts, persists and stops the machines of an
  application by identifier.
- `#[on_transition]` for handlers which run when a transition matches.

## Configuration

`ConfigurationData::with_parameters` takes, in order:

| Argument | Meaning |
| --- | --- |
| `auto_start` | Start the machine when it is built. |
| `ensemble` | Distributed coordination; not implemented, setting it is rejected. |
| `listeners` | State machine listeners attached at build time. |
| `security_enabled` | Enforce the security rules. |
| `transition_security_access_decision_manager` | Decides access for transitions. |
| `event_security_access_decision_manager` | Decides access for events. |
| `event_security_rule` | Rule evaluated for events. |
| `transition_security_rule` | Rule evaluated for transitions. |
| `verifier_enabled` | Verify the model while it is built. |
| `verifier` | Custom verifier. |
| `machine_id` | Identifier, also the key a service uses. |
| `state_machine_monitor` | Monitor notified about transitions and actions. |
| `interceptors` | Interceptors attached at build time. |

`with_all_parameters` additionally takes the transition conflict policy, the
state action policy with its timeout, and the region execution policy.

## Not supported

- **Parallel region execution**: the runtime processes the events of a machine
  one at a time. `RegionExecutionPolicy::Parallel` is rejected when the machine
  is built; `Sequential` is accepted because it describes what the runtime does.
- **Distributed machines** (`ensemble`): rejected when the machine is built.
- **Interrupting a state action by leaving the state** (`ImmediateCancel`): the
  runtime awaits a state action inline, so exiting the state cannot cancel it.
  A policy which bounds the action with a timeout (`TimeoutCancel`) is honoured.
- **Reentrant event processing**: an action must not await an event of its own
  machine, because the machine holds the slot while the action runs. Spawn the
  send instead.

## Examples

The examples live in this repository and are not shipped with the crate:

| Example | Command | Output |
| --- | --- | --- |
| Basic transitions | `cargo run -p next-web-examples --bin state_machine_basic` | `state: 3` |
| Parent and child machine | `cargo run -p next-web-examples --bin state_machine_hierarchy` | `child moved: true` |
| Service and persistence | `cargo run -p next-web-examples --bin state_machine_service` | `restored: 2` |
| Transition handler | `cargo run -p next-web-examples --bin state_machine_handler` | `audited: 1` |

## Performance

`tests/performance.rs` holds a throughput matrix of the runtime, run on demand:

```text
cargo test -p next-web-state-machine --release --test performance -- --ignored --nocapture
```

It reports nanoseconds per event for a two state cycle under these conditions:

| Scenario | What it isolates |
| --- | --- |
| sync actions, current thread | the pipeline itself, actions without an await |
| async actions, current thread | the cost of an action which awaits once |
| sync and async actions, four workers | the same work with a scheduler which can park the task |
| concurrent senders, four tasks | the contention for the single event slot of a machine |
| denied event | the path of an event which no transition accepts |
| timer driven | transitions driven by a timer, and the resolution of the platform timer |

Compare the numbers within one run on one machine; they are not a promise about
another machine. The release build matters: a debug build measures the debug
profile of this crate and of its dependencies.

## Feature flags

The state machine is reached through the umbrella crate with
`enable-state-machine`; `full` does not enable it. The umbrella re-exports this
crate as `next_web::state_machine` and its `#[on_transition]` attribute, which
this crate re-exports as well, so a project which depends on this crate directly
uses `next_web_state_machine::on_transition`.

## License

The license of the workspace which contains this crate.
