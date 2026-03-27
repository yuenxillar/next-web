use crate::config::configurers::transition_configurer::TransitionConfigurer;

pub trait InternalTransitionConfigurer<S, E>
where
    Self: TransitionConfigurer<S, E>,
{
}
