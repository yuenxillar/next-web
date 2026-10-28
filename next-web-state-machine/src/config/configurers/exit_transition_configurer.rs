use crate::config::{
    builders::state_machine_transition_configurer::StateMachineTransitionConfigurer,
    configurer_builder::ConfigurerBuilder,
};

/// Configurer of an exit point.
///
/// An exit point leaves a composite state through a transition of the enclosing
/// region.
pub trait ExitTransitionConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineTransitionConfigurer<S, E>>>,
{
    /// Sets the state the exit point belongs to.
    ///
    /// # Arguments
    /// * `source` - the source state
    fn source(&mut self, source: S) -> &mut dyn ExitTransitionConfigurer<S, E>;

    /// Sets the state the exit point continues with.
    ///
    /// # Arguments
    /// * `target` - the target state
    fn target(&mut self, target: S) -> &mut dyn ExitTransitionConfigurer<S, E>;
}
