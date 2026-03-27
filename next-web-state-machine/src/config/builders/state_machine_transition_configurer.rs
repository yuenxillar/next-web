use next_web_core::{clone_trait_object, error::BoxError, DynClone};

use crate::config::configurers::{
    choice_transition_configurer::ChoiceTransitionConfigurer,
    entry_transition_configurer::EntryTransitionConfigurer,
    exit_transition_configurer::ExitTransitionConfigurer,
    external_transition_configurer::ExternalTransitionConfigurer,
    fork_transition_configurer::ForkTransitionConfigurer,
    history_transition_configurer::HistoryTransitionConfigurer,
    internal_transition_configurer::InternalTransitionConfigurer,
    join_transition_configurer::JoinTransitionConfigurer,
    junction_transition_configurer::JunctionTransitionConfigurer,
    local_transition_configurer::LocalTransitionConfigurer,
};

pub trait StateMachineTransitionConfigurer<S, E>
where
    Self: DynClone,
{
    /// Gets a configurer for external transition.
    fn with_external(&mut self) -> Result<Box<dyn ExternalTransitionConfigurer<S, E>>, BoxError>;

    /// Gets a configurer for internal transition. Internal transition is used when action needs to be
    /// executed without causing a state transition. With internal transition source and target state is
    /// always a same and it is identical with self-transition in the absence of state entry and exit actions
    fn with_internal(&mut self) -> Result<Box<dyn InternalTransitionConfigurer<S, E>>, BoxError>;

    /// Gets a configurer for local transition. Local transition doesn’t cause exit and entry to source state if
    /// target state is a substate of a source state. Other way around, local transition doesn’t cause exit and
    /// entry to target state if target is a superstate of a source state.
    fn with_local(&mut self) -> Result<Box<dyn LocalTransitionConfigurer<S, E>>, BoxError>;

    /// Gets a configurer for transition from a choice pseudostate.
    fn with_choice(&mut self) -> Result<Box<dyn ChoiceTransitionConfigurer<S, E>>, BoxError>;

    /// Gets a configurer for transition from a junction pseudostate.
    fn with_junction(&mut self) -> Result<Box<dyn JunctionTransitionConfigurer<S, E>>, BoxError>;

    /// Gets a configurer for transition from a fork pseudostate.
    fn with_fork(&mut self) -> Result<Box<dyn ForkTransitionConfigurer<S, E>>, BoxError>;

    /// Gets a configurer for transition from a join pseudostate.
    fn with_join(&mut self) -> Result<Box<dyn JoinTransitionConfigurer<S, E>>, BoxError>;

    /// Gets a configurer for transition from an entrypoint pseudostate.
    fn with_entry(&mut self) -> Result<Box<dyn EntryTransitionConfigurer<S, E>>, BoxError>;

    /// Gets a configurer for transition from an exitpoint pseudostate.
    fn with_exit(&mut self) -> Result<Box<dyn ExitTransitionConfigurer<S, E>>, BoxError>;

    /// Gets a configurer for default history transition.
    fn with_history(&mut self) -> Result<Box<dyn HistoryTransitionConfigurer<S, E>>, BoxError>;
}

clone_trait_object!(<S, E> StateMachineTransitionConfigurer<S, E>);
