use std::{ops::Deref, sync::Arc};

use next_web_core::traits::ordered::Ordered;

use crate::{
    listener::default_composite_listener::DefaultCompositeListener,
    state::{pseudo_state_context::PseudoStateContext, pseudo_state_listener::PseudoStateListener},
};

#[derive(Clone)]
pub struct CompositePseudoStateListener<S, E> {
    base: DefaultCompositeListener<S, E>,
}

impl<S, E> CompositePseudoStateListener<S, E> {
    pub fn new() -> Self {
        Self {
            base: DefaultCompositeListener::default(),
        }
    }

    pub fn set_listeners(&mut self, listeners: Vec<Arc<dyn PseudoStateListener<S, E>>>) {
        self.base.set_listeners(listeners);
    }

    pub fn register(&mut self, listener: Arc<dyn PseudoStateListener<S, E>>) {
        self.base.register(listener);
    }
}

impl<S, E> PseudoStateListener<S, E> for CompositePseudoStateListener<S, E> {
    fn on_context(&self, context: &dyn PseudoStateContext<S, E>) {
        let mut listeners = self.base.get_listeners().clone();
        listeners.reverse();

        for listener in self.base.get_listeners().iter() {
            listener.on_context(context);
        }
    }
}

impl<S, E> Ordered for CompositePseudoStateListener<S, E> {
    fn order(&self) -> i32 {
        i32::MAX
    }
}

impl<S, E> Deref for CompositePseudoStateListener<S, E> {
    type Target = DefaultCompositeListener<S, E>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
