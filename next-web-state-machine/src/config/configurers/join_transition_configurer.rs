use crate::config::{
    builders::state_machine_transition_configurer::StateMachineTransitionConfigurer,
    configurer_builder::ConfigurerBuilder,
};

/// Configurer of a join pseudo state.
///
/// A join waits until every source state is active before it continues with the
/// target state.
pub trait JoinTransitionConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineTransitionConfigurer<S, E>>>,
{
    /// Sets the first source state of the join.
    ///
    /// # Arguments
    /// * `source` - the source state
    fn source(&mut self, source: S) -> &mut dyn JoinTransitionConfigurer<S, E>;

    /// Sets every source state of the join.
    ///
    /// # Arguments
    /// * `sources` - the source states
    fn sources(&mut self, sources: Vec<S>) -> &mut dyn JoinTransitionConfigurer<S, E>;

    /// Sets the state the join continues with once every source is active.
    ///
    /// # Arguments
    /// * `target` - the target state
    fn target(&mut self, target: S) -> &mut dyn JoinTransitionConfigurer<S, E>;
}
