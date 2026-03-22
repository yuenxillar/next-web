use std::sync::Arc;

use crate::state_machine::config::{
    action::StateMachineAction,
    builders::state_machine_transition_configurer::StateMachineTransitionConfigurer,
    configurer_builder::ConfigurerBuilder, guard::StateMachineGuard,
};

pub trait TransitionConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineTransitionConfigurer<S, E>>>,
{
    /// Specify a source state S for this Transition.
    fn source(&mut self, source: S) -> &mut dyn TransitionConfigurer<S, E>;

    /// Specify a state this transition should belong to.
    fn state(&mut self, state: S) -> &mut dyn TransitionConfigurer<S, E>;

    /// Specify event E for this Transition which will be triggered by a event trigger.
    fn event(&mut self, event: E) -> &mut dyn TransitionConfigurer<S, E>;

    /// Specify that this transition is triggered by a time.
    fn timer(&mut self, period: u64) -> &mut dyn TransitionConfigurer<S, E>;

    /// Specify that this transition is triggered once by a time after a delay.
    fn timer_once(&mut self, period: u64) -> &mut dyn TransitionConfigurer<S, E>;

    /// Specify Action for this Transition.
    fn action(
        &mut self,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn TransitionConfigurer<S, E>;

    /// Specify a Guard for this Transition.
    fn guard(
        &mut self,
        guard: Arc<dyn StateMachineGuard<S, E>>,
    ) -> &mut dyn TransitionConfigurer<S, E>;
}
