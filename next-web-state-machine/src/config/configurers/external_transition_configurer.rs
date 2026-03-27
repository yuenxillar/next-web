use crate::config::configurers::transition_configurer::TransitionConfigurer;

pub trait ExternalTransitionConfigurer<S, E>
where
    Self: TransitionConfigurer<S, E>,
{
    /// Specify a target state  S for this  Transition.
    fn target(&mut self, target: S) -> &mut dyn ExternalTransitionConfigurer<S, E>;
}
