use std::sync::Arc;

use futures::future::BoxFuture;

use crate::state_machine::{
    state::{
        pseudo_state_kind::PseudoStateKind, pseudo_state_listener::PseudoStateListener,
        StateMachineState,
    },
    state_context::StateContext,
};

/// Trait for pseudo state functionality.
pub trait PseudoState<S, E>
where
    Self: Send + Sync,
{
    /// Gets the kind of this pseudo state.
    fn get_kind(&self) -> PseudoStateKind;

    /// Called when entering this pseudo state.
    fn entry(
        &self,
        context: &dyn StateContext<S, E>,
    ) -> BoxFuture<'_, Option<Arc<dyn StateMachineState<S, E>>>>;

    /// Called when exiting this pseudo state.
    fn exit(&self, context: &dyn StateContext<S, E>) -> BoxFuture<'_, ()>;

    /// Adds a pseudo state listener.
    fn add_pseudo_state_listener(&mut self, listener: Arc<dyn PseudoStateListener<S, E>>);

    /// Sets pseudo state listeners, replacing any existing ones.
    fn set_pseudo_state_listeners(&mut self, listeners: Vec<Arc<dyn PseudoStateListener<S, E>>>);
}
