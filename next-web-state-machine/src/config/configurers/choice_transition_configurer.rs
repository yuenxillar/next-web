use std::sync::Arc;

use crate::config::{
    action::StateMachineAction,
    builders::state_machine_transition_configurer::StateMachineTransitionConfigurer,
    configurer_builder::ConfigurerBuilder, guard::StateMachineGuard,
};

/// Configurer of the choice pseudo state branches.
///
/// The branches are evaluated in order. The first branch whose guard passes is
/// taken, and the last branch is the fallback when no guard matches.
pub trait ChoiceTransitionConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineTransitionConfigurer<S, E>>>,
{
    /// Sets the state the choice branches start from.
    ///
    /// # Arguments
    /// * `source` - the choice state
    fn source(&mut self, source: S) -> &mut dyn ChoiceTransitionConfigurer<S, E>;

    /// Adds the first branch.
    ///
    /// # Arguments
    /// * `target` - the state the branch continues with
    /// * `guard` - the guard which selects the branch
    fn first(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E>;

    /// Adds the first branch with an action.
    ///
    /// # Arguments
    /// * `target` - the state the branch continues with
    /// * `guard` - the guard which selects the branch
    /// * `action` - the action executed when the branch is selected
    fn first_with_action(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E>;

    /// Adds the first branch with an action and an error action.
    ///
    /// # Arguments
    /// * `target` - the state the branch continues with
    /// * `guard` - the guard which selects the branch
    /// * `action` - the action executed when the branch is selected
    /// * `error` - the action executed when the action fails
    fn first_with_error(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
        error: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E>;

    /// Adds another branch.
    ///
    /// # Arguments
    /// * `target` - the state the branch continues with
    /// * `guard` - the guard which selects the branch
    fn then(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E>;

    /// Adds another branch with an action.
    ///
    /// # Arguments
    /// * `target` - the state the branch continues with
    /// * `guard` - the guard which selects the branch
    /// * `action` - the action executed when the branch is selected
    fn then_with_action(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E>;

    /// Adds another branch with an action and an error action.
    ///
    /// # Arguments
    /// * `target` - the state the branch continues with
    /// * `guard` - the guard which selects the branch
    /// * `action` - the action executed when the branch is selected
    /// * `error` - the action executed when the action fails
    fn then_with_error(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
        error: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E>;

    /// Adds the last branch, which is the fallback when no guard matches.
    ///
    /// # Arguments
    /// * `target` - the state the branch continues with
    fn last(&mut self, target: S) -> &mut dyn ChoiceTransitionConfigurer<S, E>;

    /// Adds the last branch with an action.
    ///
    /// # Arguments
    /// * `target` - the state the branch continues with
    /// * `action` - the action executed when the branch is selected
    fn last_with_action(
        &mut self,
        target: S,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E>;

    /// Adds the last branch with an action and an error action.
    ///
    /// # Arguments
    /// * `target` - the state the branch continues with
    /// * `action` - the action executed when the branch is selected
    /// * `error` - the action executed when the action fails
    fn last_with_error(
        &mut self,
        target: S,
        action: Arc<dyn StateMachineAction<S, E>>,
        error: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E>;
}
