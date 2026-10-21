# next-web-state-machine

A state machine for Next Web applications: states and transitions, orthogonal
regions, timers, guards and actions, a service layer which owns a machine, and
persistence of the current state.

## Quick start

```rust
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
use next_web_core::messaging::generic_message::GenericMessage;

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
        ].into()),
        Some(TransitionsData::new(vec![TransitionData::new(1, 2, 100)])),
    );

    let machine = BaseStateMachineFactory::<i32, i32>::new(Arc::new(model), None)
        .get_state_machine()
        .expect("the machine builds");

    machine.start().await.expect("the machine starts");
    machine
        .send_event(Arc::new(GenericMessage::with_payload(100)))
        .await
        .expect("the event is sent");

    assert_eq!(machine.state().map(|state| *state.id()), Some(2));
}
```

The full version of this machine is `examples/state_machine_basic.rs`; run it
with `cargo run -p next-web-examples --bin state_machine_basic`, which prints
`state: 3`.

## What it supports

- States and transitions, including internal, external and local transitions.
- Guards, entry actions, exit actions and state actions.
- Composite states: a state which owns a submachine forwards events to it.
- Orthogonal regions, fork and join.
- Deferred events, which a state postpones until it is left.
- History pseudo states.
- Timed transitions, armed when a state is entered.
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

| Example | Command | Output |
| --- | --- | --- |
| Basic transitions | `cargo run -p next-web-examples --bin state_machine_basic` | `state: 3` |
| Parent and child machine | `cargo run -p next-web-examples --bin state_machine_hierarchy` | `child moved: true` |
| Service and persistence | `cargo run -p next-web-examples --bin state_machine_service` | `restored: 2` |
| Transition handler | `cargo run -p next-web-examples --bin state_machine_handler` | `audited: 1` |

## Feature flags

The state machine is reached through the umbrella crate:
`next-web = { features = ["enable-state-machine"] }`, which re-exports this crate
as `next_web::state_machine` and its `#[on_transition]` attribute. A project
which depends on this crate directly passes `crate = next_web_state_machine` to
the attribute.

## License

The license of the workspace which contains this crate.
