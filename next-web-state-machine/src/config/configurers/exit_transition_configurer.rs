use crate::config::{
    builders::state_machine_transition_configurer::StateMachineTransitionConfigurer,
    configurer_builder::ConfigurerBuilder,
};

pub trait ExitTransitionConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineTransitionConfigurer<S, E>>>,
{
    fn source(&mut self, source: S) -> &mut dyn ExitTransitionConfigurer<S, E>;

    fn target(&mut self, target: S) -> &mut dyn ExitTransitionConfigurer<S, E>;
}
