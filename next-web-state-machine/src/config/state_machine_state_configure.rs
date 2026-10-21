use std::collections::HashSet;
use std::hash::Hash;
use std::marker::PhantomData;

/// Legacy snapshot of the states of a machine.
///
/// It only keeps the type parameters; the current path builds machines through
/// the model and the factory.
#[derive(Clone)]
pub struct StateMachineStateConfigure<S, E> {
    pub(crate) initial_state: S,
    pub(crate) states: HashSet<S>,
    pub(crate) event: PhantomData<E>,
}

impl<S, E> StateMachineStateConfigure<S, E> {
    /// Creates a snapshot with the given initial state.
    ///
    /// # Arguments
    /// * `initial_state` - the initial state
    pub fn new(initial_state: S) -> Self {
        Self {
            initial_state,
            states: HashSet::new(),
            event: Default::default(),
        }
    }

    /// Sets the initial state.
    ///
    /// # Arguments
    /// * `state` - the initial state
    pub fn set_initial_state(mut self, state: S) -> Self {
        self.initial_state = state;
        self
    }

    /// Sets the states of the snapshot.
    ///
    /// # Arguments
    /// * `states` - the states
    pub fn set_states<T>(mut self, states: T) -> Self
    where
        S: Hash + Eq,
        T: IntoIterator<Item = S>,
    {
        self.states = states.into_iter().collect();
        self
    }

    /// Gets the states of the snapshot.
    pub fn states(&self) -> &HashSet<S> {
        &self.states
    }

    /// Gets the initial state of the snapshot.
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
