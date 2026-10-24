use crate::config::configurers::transition_configurer::TransitionConfigurer;

/// Configurer of an internal transition.
///
/// An internal transition runs its actions without leaving the source state.
pub trait InternalTransitionConfigurer<S, E>
where
    Self: TransitionConfigurer<S, E>,
{
}
