use next_web_core::{DynClone, clone_trait_object, error::BoxError};

use crate::config::configurers::model_configurer::ModelConfigurer;

/// Configurer interface exposing model.
pub trait StateMachineModelConfigurer<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
    Self: DynClone,
{
    ///  Gets a configurer for model.
    fn with_model(&mut self) -> Result<Box<dyn ModelConfigurer<S, E>>, BoxError>;
}

clone_trait_object!(<S, E> StateMachineModelConfigurer<S, E>);
