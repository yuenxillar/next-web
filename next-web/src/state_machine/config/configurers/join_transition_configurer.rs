use crate::state_machine::config::{
    builders::state_machine_transition_configurer::StateMachineTransitionConfigurer,
    configurer_builder::ConfigurerBuilder,
};

pub trait JoinTransitionConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineTransitionConfigurer<S, E>>>,
{
    fn source(&mut self, source: S) -> &mut dyn JoinTransitionConfigurer<S, E>;

    fn sources(&mut self, sources: Vec<S>) -> &mut dyn JoinTransitionConfigurer<S, E>;

    fn target(&mut self, target: S) -> &mut dyn JoinTransitionConfigurer<S, E>;
}
