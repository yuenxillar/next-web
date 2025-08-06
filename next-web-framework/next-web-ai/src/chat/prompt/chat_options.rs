use next_web_core::DynClone;

use crate::model::model_options::ModelOptions;

pub trait ChatOptions: DynClone + Send + Sync
where
    Self: ModelOptions,
{
}

next_web_core::clone_trait_object!(ChatOptions);
