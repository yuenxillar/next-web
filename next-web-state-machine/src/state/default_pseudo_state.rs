//! The default, pass-through pseudo state.

use std::sync::Arc;

use futures::future::BoxFuture;
use next_web_core::error::BoxError;

use crate::state::StateMachineState;
use crate::state::composite_pseudo_state_listener::CompositePseudoStateListener;
use crate::state::pseudo_state::PseudoState;
use crate::state::pseudo_state_context::PseudoStateContext;
use crate::state::pseudo_state_kind::PseudoStateKind;
use crate::state::pseudo_state_listener::PseudoStateListener;
use crate::state_context::StateContext;

/// Default implementation of a `PseudoState`.
///
/// It is used for the `INITIAL`, `END` and `TERMINATE` kinds, which only mark
/// a state and do not intercept anything in the Java implementation either.
pub struct DefaultPseudoState<S, E> {
    kind: PseudoStateKind,
    listeners: CompositePseudoStateListener<S, E>,
}

impl<S, E> DefaultPseudoState<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Instantiates a new pseudo state of the given kind.
    ///
    /// # Arguments
    /// * `kind` - the kind of the pseudo state
    pub fn new(kind: PseudoStateKind) -> Self {
        Self {
            kind,
            listeners: CompositePseudoStateListener::new(),
        }
    }
}

impl<S, E> PseudoState<S, E> for DefaultPseudoState<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn get_kind(&self) -> PseudoStateKind {
        self.kind
    }

    fn entry<'a>(
        &'a self,
        _context: &'a dyn StateContext<S, E>,
    ) -> BoxFuture<'a, Option<Arc<dyn StateMachineState<S, E>>>> {
        Box::pin(async move { None })
    }

    fn exit<'a>(
        &'a self,
        _context: &'a dyn StateContext<S, E>,
    ) -> BoxFuture<'a, Result<(), BoxError>> {
        Box::pin(async move { Ok(()) })
    }

    fn add_pseudo_state_listener(&self, listener: Arc<dyn PseudoStateListener<S, E>>) {
        self.listeners.register(listener);
    }

    fn set_pseudo_state_listeners(&self, listeners: Vec<Arc<dyn PseudoStateListener<S, E>>>) {
        self.listeners.set_listeners(listeners);
    }

    fn notify_context(&self, context: &dyn PseudoStateContext<S, E>) {
        self.listeners.on_context(context);
    }
}
