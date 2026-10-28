# Changelog

All notable changes to `next-web-state-machine` are listed here.

## 0.2.0

### Added

- A service layer which owns the machines of an application by identifier:
  `service::StateMachineService` and `service::DefaultStateMachineService`, also
  re-exported from `service`. It builds a machine on demand, restores a persisted
  one, starts it, and stops and persists it when it is released.
- Transition handlers: `handler::on_transition::OnTransition` runs an action when
  a transition matches a source, a target or an event, registered through
  `StateMachine::add_transition_handler`. The `#[on_transition]` attribute
  generates the handler; it targets `::next_web::state_machine` by default and is
  re-exported from this crate, with `crate = ..` for a project which depends on
  this crate directly.
- `StateMachineErrorKind::MachineNotFound`, which a store reports when it has
  nothing for a key, and which a service uses to build a fresh machine.
- `ExtendedState::set`, `StateMachine::restore_context` and
  `StateMachine::reset_to`, so a machine can be restored from a context.
- `Region::event_lock`, the single slot through which events and timer triggers
  are serialized.

### Changed

- Events are carried as `Arc<dyn Message<E>>` instead of `Box<dyn Message<E>>`.
- `Region::send_event` returns `Arc<dyn StateMachineEventResult<S, E>>`.
- `ExtendedState::get_variables` and `ExtendedState::get` return owned values,
  and `set_extended_state_change_listener` takes `&self`.
- `StateMachinePersister` is implemented in terms of `StateMachinePersist`.
- The state do action policy of the configuration reaches the built states.
- `RegionExecutionPolicy::Parallel` is rejected when a machine is built, and
  `ImmediateCancel` is documented as not interruptible.

### Fixed

- The events of one machine are processed one at a time, so two concurrent
  `send_event` calls cannot interleave the exit and entry steps.
- Configurers are keyed by their type and released after a build, so a builder
  no longer leaks them and no longer overwrites one type with another.
- A restored machine keeps its extended state variables, and starting it
  continues in the restored state instead of re-entering the initial one.
- A failed post transition interceptor — for example a failing handler — is
  reported to the caller instead of only being logged.
- Releasing a machine whose persist failed still stops it, so its triggers are
  disarmed.
- Transitions declared inside a submachine are no longer dropped, and an event
  which no transition of the current state accepts is forwarded to the
  submachine of that state.
- A guard which refuses every candidate no longer looks like an accepted event.
- A timer starts counting when the source state of its transition is entered, and
  it fires for the first time after its period. Before, every timer was armed
  when the machine started and its first firing was immediate, so a timer out of
  a state entered later had usually expired before the machine arrived.
- Leaving the state which owns a timer no longer aborts the transition which that
  timer is running: the timer task stops at its next tick instead of cancelling
  itself in the middle of the transition.

## Migration from 0.1

| 0.1 | 0.2.0 |
| --- | --- |
| `send_event(Box<dyn Message<E>>)` | `send_event(Arc<dyn Message<E>>)` |
| `send_event` returns `Box<dyn StateMachineEventResult<S, E>>` | returns `Arc<dyn StateMachineEventResult<S, E>>` |
| `send_events(Vec<Box<dyn Message<E>>>) -> Box<dyn StateMachineEventResult<S, E>>` | `send_events(Vec<Arc<dyn Message<E>>>) -> Vec<Arc<dyn StateMachineEventResult<S, E>>>` |
| `Region::state() -> Option<&dyn StateMachineState<S, E>>` | `-> Option<Arc<dyn StateMachineState<S, E>>>` |
| `Region::states() -> Vec<&dyn StateMachineState<S, E>>` | `-> Vec<Arc<dyn StateMachineState<S, E>>>` |
| `StateMachine::set_state_machine_error(&mut self, Box<dyn Error + Send>)` | `&self, BoxError` |
| `ExtendedState::get_variables() -> &HashMap<K, AnyValue>` | `-> HashMap<K, AnyValue>` (a copy) |
| `ExtendedState::get(&K) -> Option<&AnyValue>` | `-> Option<AnyValue>` |
| — | `ExtendedState::set(&self, K, AnyValue)` |
| — | `Region::event_lock() -> &tokio::sync::Mutex<()>` (a new required method) |
| — | `StateMachine::restore_context`, `StateMachine::reset_to` (which also disables the initial transition) |
| Persistence wrote the state only | The extended state variables are persisted and restored as well |
| — | `StateMachineErrorKind::MachineNotFound` (match on it in your store) |

A custom `Region` implementation has to provide `event_lock`; returning a mutex
which is held by `send_event` is what the built-in machine does.

A custom `StateMachineRuntimePersister` is still needed if the runtime persistence
of a machine is configured: this crate does not ship a default implementation.
