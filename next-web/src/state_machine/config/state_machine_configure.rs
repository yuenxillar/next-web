// use std::collections::HashSet;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct StateMachineConfigure<S, E> {
    pub(crate) state: PhantomData<S>,
    pub(crate) event: PhantomData<E>,
}

impl<S, E> StateMachineConfigure<S, E> {
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
