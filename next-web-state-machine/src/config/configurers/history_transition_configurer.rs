use crate::config::{
    builders::state_machine_transition_configurer::StateMachineTransitionConfigurer,
    configurer_builder::ConfigurerBuilder,
};

/// Configurer of the default history transition of a composite state.
pub trait HistoryTransitionConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineTransitionConfigurer<S, E>>>,
{
    /// Sets the composite state the history belongs to.
    ///
    /// # Arguments
    /// * `source` - the composite state
    fn source(&mut self, source: S) -> &mut dyn HistoryTransitionConfigurer<S, E>;

    /// Sets the state entered when no history was recorded yet.
    ///
    /// # Arguments
    /// * `target` - the default target state
    fn target(&mut self, target: S) -> &mut dyn HistoryTransitionConfigurer<S, E>;
}
