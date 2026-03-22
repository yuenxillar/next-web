use std::sync::Arc;

use crate::state_machine::config::{
    action::StateMachineAction,
    builders::state_machine_transition_configurer::StateMachineTransitionConfigurer,
    configurer_builder::ConfigurerBuilder, guard::StateMachineGuard,
};

pub trait JunctionTransitionConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineTransitionConfigurer<S, E>>>,
{
    fn source(&mut self, source: S) -> &mut dyn JunctionTransitionConfigurer<S, E>;

    fn first(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E>;

    fn first_with_action(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E>;

    fn first_with_error(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
        error: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E>;

    fn then(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E>;

    fn then_with_action(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E>;

    fn then_with_error(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
        error: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E>;

    fn last(&mut self, target: S) -> &mut dyn JunctionTransitionConfigurer<S, E>;

    fn last_with_action(
        &mut self,
        target: S,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E>;

    fn last_with_error(
        &mut self,
        target: S,
        action: Arc<dyn StateMachineAction<S, E>>,
        error: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E>;
}
