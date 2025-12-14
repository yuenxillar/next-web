use std::sync::Arc;

use next_web_core::anys::any_value::AnyValue;

use super::{EventMessage, StateMachine, Transition};

#[derive(Clone, PartialEq, Eq, Hash)]

pub struct StateMachineKey<S, E>(pub String, pub S, pub S, pub E);

impl<S, E> Into<StateMachineKey<S, E>> for (String, S, S, E) {
    fn into(self) -> StateMachineKey<S, E> {
        StateMachineKey(self.0, self.1, self.2, self.3)
    }
}

impl<S, E> Into<StateMachineKey<S, E>> for (String, Transition<S, E>) {
    fn into(self) -> StateMachineKey<S, E> {
        let state = self.1;
        (
            self.0,
            state.source,
            state.target,
            state.event,
        )
            .into()
    }
}

#[derive(Clone)]
pub struct StateContext<S, E> {
    pub(crate) message: EventMessage<E>,
    pub(crate) transition: Transition<S, E>,
    pub(crate) state_machine: Arc<StateMachine<S, E>>,
}

impl<S, E> StateContext<S, E>
where
    S: Send,
    E: Send,
{
    pub fn message(&self) -> &EventMessage<E> {
        &self.message
    }

    pub fn payload(&self) -> Option<&AnyValue> {
        self.message.payload.as_ref()
    }

    pub fn state_machine(&self) -> &StateMachine<S, E> {
        &self.state_machine
    }

    pub fn transition(&self) -> &Transition<S, E> {
        &self.transition
    }

    pub fn source(&self) -> &S {
        &self.transition.source
    }

    pub fn target(&self) -> &S {
        &self.transition.target
    }

    pub fn event(&self) -> &E {
        &self.transition.event
    }
}
