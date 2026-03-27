use std::sync::Arc;

use next_web_core::traits::message::Message;
use uuid::Uuid;

use crate::{
    extended_state::ExtendedState, state::StateMachineState,
    support::base_state_machine::BaseStateMachine, transition::StateMachineTransition,
};

pub struct DefaultStateMachie<S, E> {
    delegate: BaseStateMachine<S, E>,
}

impl<S, E> DefaultStateMachie<S, E>
where
    S: Send + Sync,
    S: 'static,
    E: Send + Sync,
    E: 'static,
{
    pub fn new<T1, T2>(
        states: T1,
        transitions: T2,
        initial_state: Arc<dyn StateMachineState<S, E>>,
    ) -> Self
    where
        T1: IntoIterator<IntoIter = Vec<Arc<dyn StateMachineState<S, E>>>>,
        T2: IntoIterator<IntoIter = Vec<Arc<dyn StateMachineTransition<S, E>>>>,
    {
        Self {
            delegate: BaseStateMachine::new(
                states.into_iter(),
                transitions.into_iter(),
                initial_state,
            ),
        }
    }

    pub fn witt_extended_state<T1, T2>(
        states: T1,
        transitions: T2,
        initial_state: Arc<dyn StateMachineState<S, E>>,
        extended_state: Arc<dyn ExtendedState>,
    ) -> Self
    where
        T1: IntoIterator<IntoIter = Vec<Arc<dyn StateMachineState<S, E>>>>,
        T2: IntoIterator<IntoIter = Vec<Arc<dyn StateMachineTransition<S, E>>>>,
    {
        Self {
            delegate: BaseStateMachine::with_extended_state(
                states.into_iter(),
                transitions.into_iter(),
                initial_state,
                extended_state,
            ),
        }
    }

    pub fn witt_all<T1, T2>(
        states: T1,
        transitions: T2,
        initial_state: Arc<dyn StateMachineState<S, E>>,
        initial_transition: Arc<dyn StateMachineTransition<S, E>>,
        initial_event: Box<dyn Message<E>>,
        extended_state: Arc<dyn ExtendedState>,
        uuid: Uuid,
    ) -> Self
    where
        T1: IntoIterator<IntoIter = Vec<Arc<dyn StateMachineState<S, E>>>>,
        T2: IntoIterator<IntoIter = Vec<Arc<dyn StateMachineTransition<S, E>>>>,
    {
        Self {
            delegate: BaseStateMachine::with_all(
                states.into_iter(),
                transitions.into_iter(),
                initial_state,
                Some(initial_transition),
                Some(initial_event),
                extended_state,
                Some(uuid),
            ),
        }
    }
}
