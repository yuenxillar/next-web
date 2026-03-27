use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;

use crate::listener::ordered_composite::OrderedComposite;
use crate::monitor::state_machine_monitor::StateMachineMonitor;
use crate::state_context::StateContext;
use crate::transition::StateMachineTransition;
use crate::StateMachine;

/// Implementation of a `StateMachineMonitor` backed by multiple monitors.
///
/// This composite monitor dispatches monitoring events to all registered monitors
/// in reverse order (last registered gets first chance).
#[derive(Clone)]
pub struct CompositeStateMachineMonitor<S, E> {
    base: OrderedComposite<Arc<dyn StateMachineMonitor<S, E>>>,
}

impl<S, E> StateMachineMonitor<S, E> for CompositeStateMachineMonitor<S, E> {
    fn transition(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        transition: &dyn StateMachineTransition<S, E>,
        duration: Duration,
    ) {
        for monitor in self.base.ordered.iter().rev() {
            monitor.transition(state_machine, transition, duration);
        }
    }

    /// Notified duration of a particular action.
    fn action(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        action: &dyn Fn(&dyn StateContext<S, E>) -> BoxFuture<'static, ()>,
        duration: Duration,
    ) {
        for monitor in self.base.ordered.iter().rev() {
            monitor.action(state_machine, action, duration);
        }
    }
}

impl<S, E> Default for CompositeStateMachineMonitor<S, E> {
    fn default() -> Self {
        Self {
            base: Default::default(),
        }
    }
}
