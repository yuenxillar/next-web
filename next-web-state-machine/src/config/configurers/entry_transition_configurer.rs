use crate::config::{
    builders::state_machine_transition_configurer::StateMachineTransitionConfigurer,
    configurer_builder::ConfigurerBuilder,
};

/// Configurer of an entry point.
///
/// An entry point enters a composite state through a transition of the enclosing
/// region.
pub trait EntryTransitionConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineTransitionConfigurer<S, E>>>,
{
    /// Sets the state the entry point belongs to.
    ///
    /// # Arguments
    /// * `source` - the source state
    fn source(&mut self, source: S) -> &mut dyn EntryTransitionConfigurer<S, E>;

    /// Sets the state the entry point continues with.
    ///
    /// # Arguments
    /// * `target` - the target state
    fn target(&mut self, target: S) -> &mut dyn EntryTransitionConfigurer<S, E>;
}
