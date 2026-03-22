use std::sync::Arc;

use crate::state_machine::{
    config::{
        action::StateMachineAction,
        builders::state_machine_state_configurer::StateMachineStateConfigurer,
        configurer_builder::ConfigurerBuilder, state_machine_factory::StateMachineFactory,
    },
    BoxedStateAction, StateMachine,
};

/// Base StateConfigurer interface for configuring States.
pub trait StateConfigurer<S, E>
where
    Self: ConfigurerBuilder<Box<dyn StateMachineStateConfigurer<S, E>>>,
    S: Send + Sync,
    E: Send + Sync,
{
    /// Specify a initial state S.
    fn initial(&mut self, initial: S) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a initial state S with an Action to be executed with it.
    /// Action can be i. e. used to init extended variables.
    fn initial_with_action(
        &mut self,
        initial: S,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a states configured by this configurer instance to be substates of state S.
    fn parent(&mut self, state: S) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a region for these states configured by this configurer instance.
    fn region(&mut self, id: String) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S.
    fn state(&mut self, state: S) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S and its relation with a given machine as substate machine.
    fn state_with_machine(
        &mut self,
        state: S,
        state_machine: Arc<dyn StateMachine<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S and its relation with a given machine as substate machine factory.
    fn state_with_factory(
        &mut self,
        state: S,
        state_machine: Arc<dyn StateMachineFactory<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S with state Actions.
    fn state_with_actions(
        &mut self,
        state: S,
        actions: Vec<Arc<dyn StateMachineAction<S, E>>>,
    ) -> &mut dyn StateConfigurer<S, E>;

    fn state_with_action(
        &mut self,
        state: S,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S with state behaviour Action.
    fn state_do(
        &mut self,
        state: S,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E>;

    fn state_do_with_error(
        &mut self,
        state: S,
        action: Arc<dyn StateMachineAction<S, E>>,
        error: Option<Arc<dyn StateMachineAction<S, E>>>,
    ) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S with state behaviour Function.
    fn state_do_function(
        &mut self,
        state: S,
        action: BoxedStateAction<S, E>,
    ) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S with state entry Function.
    fn state_entry_function(
        &mut self,
        state: S,
        action: BoxedStateAction<S, E>,
    ) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S with state exit Function.
    fn state_exit_function(
        &mut self,
        state: S,
        action: BoxedStateAction<S, E>,
    ) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S with entry and exit Actions.
    fn state_with_entry_and_exit_functions(
        &mut self,
        state: S,
        entry_actions: Vec<Arc<dyn StateMachineAction<S, E>>>,
        exit_actions: Vec<Arc<dyn StateMachineAction<S, E>>>,
    ) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S with entry and exit Action.
    fn state_with_entry_and_exit_function(
        &mut self,
        state: S,
        entry_action: Arc<dyn StateMachineAction<S, E>>,
        exit_action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S with state entry Action.
    fn state_entry(
        &mut self,
        state: S,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S with state entry Action and error Action callback.
    fn state_entry_with_error(
        &mut self,
        state: S,
        action: Arc<dyn StateMachineAction<S, E>>,
        error: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S with state exit Action.
    fn state_exit(
        &mut self,
        state: S,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S with state exit Action and error Action callback.
    fn state_exity_with_error(
        &mut self,
        state: S,
        action: Arc<dyn StateMachineAction<S, E>>,
        error: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S with a deferred events E.
    fn state_deferred(&mut self, state: S, deferred: Vec<E>) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a states S.
    fn states(&mut self, states: Vec<S>) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S to be end state. This method can be called for each state to be marked as end state.
    fn end(&mut self, end: S) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S to be choice pseudo state.
    fn choice(&mut self, choice: S) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S to be junction pseudo state.
    fn junction(&mut self, junction: S) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S to be fork pseudo state.
    fn fork(&mut self, fork: S) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S to be join pseudo state.
    fn join(&mut self, join: S) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S to be history pseudo state.
    fn history(&mut self, history: S, ty: History) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S to be entrypoint pseudo state.
    fn entry(&mut self, entry: S) -> &mut dyn StateConfigurer<S, E>;

    /// Specify a state S to be exitpoint pseudo state.
    fn exit(&mut self, exit: S) -> &mut dyn StateConfigurer<S, E>;
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum History {
    /// Shallow history is a pseudo state representing the most
    /// recent substate of a submachine.
    Shallow,

    ///  Deep history is a shallow history recursively reactivating
    /// the substates of the most recent substate.
    Deep,
}
