//! A monitor that fans out transition and action timings.

use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

use crate::BoxedStateAction;
use crate::StateMachine;
use crate::monitor::state_machine_monitor::StateMachineMonitor;
use crate::support::lock_util::{read, write};
use crate::transition::StateMachineTransition;

/// A composite of [`StateMachineMonitor`]s.
///
/// Registration replaces the monitor list copy-on-write, so a notification
/// clones one [`Arc`] instead of every monitor, and monitors can be registered
/// while the machine runs.
pub struct CompositeStateMachineMonitor<S, E> {
    monitors: RwLock<Arc<Vec<Arc<dyn StateMachineMonitor<S, E>>>>>,
}

impl<S, E> Default for CompositeStateMachineMonitor<S, E> {
    fn default() -> Self {
        Self {
            monitors: RwLock::new(Arc::new(Vec::new())),
        }
    }
}

impl<S, E> Clone for CompositeStateMachineMonitor<S, E> {
    fn clone(&self) -> Self {
        Self {
            monitors: RwLock::new(read(&self.monitors).clone()),
        }
    }
}

impl<S, E> CompositeStateMachineMonitor<S, E> {
    /// Creates an empty composite monitor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a monitor.
    ///
    /// # Arguments
    /// * `monitor` - the monitor to register
    pub fn register(&self, monitor: Arc<dyn StateMachineMonitor<S, E>>) {
        let mut guard = write(&self.monitors);
        if guard.iter().any(|item| Arc::ptr_eq(item, &monitor)) {
            return;
        }
        let mut next = Vec::with_capacity(guard.len() + 1);
        next.extend(guard.iter().cloned());
        next.push(monitor);
        *guard = Arc::new(next);
    }

    /// Unregisters a monitor.
    ///
    /// # Arguments
    /// * `monitor` - the monitor to unregister
    pub fn unregister(&self, monitor: Arc<dyn StateMachineMonitor<S, E>>) {
        let mut guard = write(&self.monitors);
        let next: Vec<_> = guard
            .iter()
            .filter(|item| !Arc::ptr_eq(item, &monitor))
            .cloned()
            .collect();
        *guard = Arc::new(next);
    }

    /// Gets a snapshot of the registered monitors.
    pub fn snapshot(&self) -> Arc<Vec<Arc<dyn StateMachineMonitor<S, E>>>> {
        read(&self.monitors).clone()
    }
}

impl<S, E> StateMachineMonitor<S, E> for CompositeStateMachineMonitor<S, E> {
    fn transition(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        transition: &dyn StateMachineTransition<S, E>,
        duration: Duration,
    ) {
        let monitors = self.snapshot();
        for monitor in monitors.iter().rev() {
            monitor.transition(state_machine, transition, duration);
        }
    }

    fn action(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        action: &BoxedStateAction<S, E>,
        duration: Duration,
    ) {
        let monitors = self.snapshot();
        for monitor in monitors.iter().rev() {
            monitor.action(state_machine, action, duration);
        }
    }
}
