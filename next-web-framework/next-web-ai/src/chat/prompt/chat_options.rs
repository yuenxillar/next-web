use next_web_core::DynClone;

use crate::model::model_options::ModelOptions;

pub trait ChatOptions
where
    Self: DynClone + ModelOptions,
{
    fn get_model(&self) -> &str;

    fn get_frequency_penalty(&self) -> f64;

    fn get_max_tokens(&self) -> u64;

    fn get_presence_penalty(&self) -> f64;

    fn get_stop_sequences(&self) -> Vec<String>;

    fn get_temperature(&self) -> f64;

    fn get_top_k(&self) -> u64;

    fn get_top_p(&self) -> u64;
}

next_web_core::clone_trait_object!(ChatOptions);
