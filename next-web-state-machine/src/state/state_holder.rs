use std::sync::Arc;

use crate::state::StateMachineState;

///  Utility class using holder pattern to keep a {@link State} reference.
pub struct StateHolder<S, E> {
    state: Arc<dyn StateMachineState<S, E>>,
}

impl<S, E> StateHolder<S, E> {
    pub fn new<T>(state: Arc<dyn StateMachineState<S, E>>) -> Self {
        StateHolder { state }
    }

    pub fn get_state(&self) -> &dyn StateMachineState<S, E> {
        self.state.as_ref()
    }

    pub fn set_state(&mut self, state: Arc<dyn StateMachineState<S, E>>) {
        self.state = state;
    }
}
