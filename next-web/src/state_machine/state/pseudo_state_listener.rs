use next_web_core::traits::ordered::Ordered;

use crate::state_machine::state::pseudo_state_context::PseudoStateContext;

/// Listener for pseudo state events.
pub trait PseudoStateListener<S, E>
where
    Self: Ordered,
    Self: Send + Sync,
{
    /// Called when pseudo state context is created or updated.
    fn on_context(&self, context: &dyn PseudoStateContext<S, E>);
}
