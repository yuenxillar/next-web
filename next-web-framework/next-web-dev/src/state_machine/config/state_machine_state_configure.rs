use std::collections::HashSet;
use std::hash::Hash;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct StateMachineStateConfigure<S, E> {
    pub(crate) initial_state: S,
    pub(crate) states: HashSet<S>,
    pub(crate) event: PhantomData<E>,
}

impl<S, E> StateMachineStateConfigure<S, E> {
    pub fn new(initial_state: S) -> Self {
        Self {
            initial_state,
            states: HashSet::new(),
            event: Default::default(),
        }
    }
    pub fn set_initial_state(mut self, state: S) -> Self {
        self.initial_state = state;
        self
    }

    pub fn set_states<T>(mut self, states: T) -> Self
    where
        S: Hash + Eq,
        T: IntoIterator<Item = S>,
    {
        self.states = states.into_iter().collect();
        self
    }

    pub fn states(&self) -> &HashSet<S> {
        &self.states
    }

    pub fn initial_state(&self) -> &S {
        &self.initial_state
    }
}

impl<S, E> Default for StateMachineStateConfigure<S, E>
where
    S: Default,
{
    fn default() -> Self {
        Self {
            initial_state: Default::default(),
            states: Default::default(),
            event: Default::default(),
        }
    }
}
