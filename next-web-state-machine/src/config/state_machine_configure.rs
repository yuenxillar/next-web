// use std::collections::HashSet;
use std::marker::PhantomData;

/// Legacy snapshot of a machine configuration.
///
/// It only keeps the type parameters; the current path builds machines through
/// the model and the factory.
#[derive(Clone)]
pub struct StateMachineConfigure<S, E> {
    pub(crate) state: PhantomData<S>,
    pub(crate) event: PhantomData<E>,
}

impl<S, E> StateMachineConfigure<S, E> {
    /// Creates an empty snapshot.
    pub fn new() -> Self {
        Self {
            state: Default::default(),
            event: Default::default(),
        }
    }
}

impl<S, E> Default for StateMachineConfigure<S, E> {
    fn default() -> Self {
        Self {
            state: Default::default(),
            event: Default::default(),
        }
    }
}
