use crate::state_machine::state::composite_pseudo_state_listener::CompositePseudoStateListener;
use crate::state_machine::state::pseudo_state::PseudoState;
use crate::state_machine::state::pseudo_state_context::PseudoStateContext;
use crate::state_machine::state::pseudo_state_kind::PseudoStateKind;
use crate::state_machine::state::pseudo_state_listener::PseudoStateListener;
use crate::state_machine::state::StateMachineState;
use crate::state_machine::state_context::StateContext;

use futures::future::BoxFuture;
use std::sync::Arc;

// impl<S, E> PseudoState<S, E> for DefaultPseudoState<S, E> {}
/// Default implementation of a `PseudoState`.
pub struct DefaultPseudoState<S, E> {
    kind: PseudoStateKind,
    listeners: CompositePseudoStateListener<S, E>,
}

impl<S, E> DefaultPseudoState<S, E> {
    /// Instantiates a new abstract pseudo state.
    pub fn new(kind: PseudoStateKind) -> Self {
        Self {
            kind,
            listeners: CompositePseudoStateListener::new(),
        }
    }

    /// Notify all `PseudoStateListener`s of a new context.
    fn notify_context(&self, context: &dyn PseudoStateContext<S, E>) {
        self.listeners.on_context(context);
    }
}

impl<S, E> PseudoState<S, E> for DefaultPseudoState<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    fn get_kind(&self) -> PseudoStateKind {
        self.kind
    }

    #[allow(unused_variables)]
    fn entry(
        &self,
        context: &dyn StateContext<S, E>,
    ) -> BoxFuture<'_, Option<Arc<dyn StateMachineState<S, E>>>> {
        Box::pin(async move { None })
    }

    #[allow(unused_variables)]
    fn exit(&self, context: &dyn StateContext<S, E>) -> BoxFuture<'_, ()> {
        Box::pin(async move {})
    }

    fn add_pseudo_state_listener(&mut self, listener: Arc<dyn PseudoStateListener<S, E>>) {
        self.listeners.register(listener);
    }

    fn set_pseudo_state_listeners(&mut self, listeners: Vec<Arc<dyn PseudoStateListener<S, E>>>) {
        self.listeners.set_listeners(listeners);
    }
}
