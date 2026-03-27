use next_web_core::{clone_trait_object, error::BoxError, DynClone};

use crate::config::configurers::state_configurer::StateConfigurer;

/// Configurer interface exposing states.
pub trait StateMachineStateConfigurer<S, E>: DynClone {
    /// Gets a configurer for states
    fn with_states(&mut self) -> Result<Box<dyn StateConfigurer<S, E>>, BoxError>;
}

clone_trait_object!(<S, E> StateMachineStateConfigurer<S, E>);
