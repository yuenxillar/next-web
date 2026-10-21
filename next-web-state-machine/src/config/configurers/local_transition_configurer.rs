/// Configurer of a local transition.
///
/// A local transition does not exit the source state when the target is its
/// substate, and does not enter the target when the target is its superstate.
pub trait LocalTransitionConfigurer<S, E> {
    /// Specify a target state for this transition.
    fn target(&mut self, target: S) -> &mut dyn LocalTransitionConfigurer<S, E>;
}
