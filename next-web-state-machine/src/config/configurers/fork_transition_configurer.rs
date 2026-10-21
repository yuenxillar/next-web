use crate::config::{
    builders::state_machine_transition_configurer::StateMachineTransitionConfigurer,
    configurer_builder::ConfigurerBuilder,
};

/// Configurer of a fork pseudo state.
///
/// A fork activates several regions of a composite state at once.
pub trait ForkTransitionConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineTransitionConfigurer<S, E>>>,
{
    /// Sets the state the fork starts from.
    ///
    /// # Arguments
    /// * `source` - the source state
    fn source(&mut self, source: S) -> &mut dyn ForkTransitionConfigurer<S, E>;

    /// Sets the state the fork continues with.
    ///
    /// # Arguments
    /// * `target` - the target state
    fn target(&mut self, target: S) -> &mut dyn ForkTransitionConfigurer<S, E>;
}
