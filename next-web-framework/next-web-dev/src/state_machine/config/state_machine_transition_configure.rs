use crate::state_machine::{StateMachineAction, Transition};

#[derive(Clone)]
pub struct StateMachineTransitionConfigure<S, E> {
    pub(crate) inner: Vec<ExternalTransitionConfigure<S, E>>,
}

impl<S, E> StateMachineTransitionConfigure<S, E> {
    pub fn with(
        mut self,
        source: S,
        target: S,
        event: E,
        action: Box<dyn StateMachineAction<S, E>>,
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

#[derive(Clone)]
pub struct ExternalTransitionConfigure<S, E> {
    pub source: S,
    pub target: S,
    pub event: E,
    action: Box<dyn StateMachineAction<S, E>>,
}

impl<S, E> ExternalTransitionConfigure<S, E>
where
    S: Clone,
    E: Clone
{
    pub fn transition(&self) -> Transition<S, E> {
        self.action.transition()
    }

    pub fn action(&self) ->  Box<dyn StateMachineAction<S, E>> {
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
