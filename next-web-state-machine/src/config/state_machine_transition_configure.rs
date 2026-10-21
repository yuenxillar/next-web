use std::sync::Arc;

use crate::config::action::StateMachineAction;

/// Legacy snapshot of the transitions of a machine.
///
/// It only keeps external transitions; the current path builds machines through
/// the model and the factory.
#[derive(Clone)]
pub struct StateMachineTransitionConfigure<S, E> {
    pub(crate) inner: Vec<ExternalTransitionConfigure<S, E>>,
}

impl<S, E> StateMachineTransitionConfigure<S, E> {
    /// Appends an external transition.
    ///
    /// # Arguments
    /// * `source` - the source state
    /// * `target` - the target state
    /// * `event` - the event which fires the transition
    /// * `action` - the action of the transition
    pub fn with(
        mut self,
        source: S,
        target: S,
        event: E,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> Self {
        self.inner.push(ExternalTransitionConfigure {
            source,
            target,
            event,
            action,
        });
        self
    }
}

/// Legacy snapshot of one external transition.
#[derive(Clone)]
pub struct ExternalTransitionConfigure<S, E> {
    /// The source state.
    pub source: S,
    /// The target state.
    pub target: S,
    /// The event which fires the transition.
    pub event: E,
    action: Arc<dyn StateMachineAction<S, E>>,
}

impl<S, E> ExternalTransitionConfigure<S, E>
where
    S: Clone,
    E: Clone,
{
    /// Gets the action of the transition.
    pub fn action(&self) -> Arc<dyn StateMachineAction<S, E>> {
        self.action.clone()
    }
}

impl<S, E> Default for StateMachineTransitionConfigure<S, E> {
    fn default() -> Self {
        Self {
            inner: Vec::with_capacity(4),
        }
    }
}
