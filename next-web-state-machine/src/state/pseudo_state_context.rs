use crate::state::pseudo_state::PseudoState;

/// Context object using in PseudoStateListener.
pub trait PseudoStateContext<S, E> {
    ///  Gets the pseudo state.
    fn get_pseudo_state(&self) -> Option<&dyn PseudoState<S, E>>;

    ///  Gets the pseudo action.
    fn get_pseudo_action(&self) -> PseudoAction;
}

/// The PseudoAction enumeration.
pub enum PseudoAction {
    /// Indication that states has been joined.
    JoinCompleted,
}
