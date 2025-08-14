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

    fn get_stop_sequences(&self) -> &Vec<String>;

    fn get_temperature(&self) -> f64;

    fn get_top_k(&self) -> u64;

    fn get_top_p(&self) -> u64;

    fn set_model(&mut self, model: &str);

    fn set_frequency_penalty(&mut self, frequency_penalty: f64);

    fn set_max_tokens(&mut self, max_tokens: u64);

    fn set_presence_penalty(&mut self, presence_penalty: f64);

    fn set_stop_sequences(&mut self, stop_sequences: Vec<String>);

    fn set_temperature(&mut self, temperature: f64);

    fn set_top_k(&mut self, top_k: u64);

    fn set_top_p(&mut self, top_p: u64);
}

next_web_core::clone_trait_object!(ChatOptions);


#[derive(Clone, Default)]
pub struct DefaultChatOptions {
    pub model: String,
    pub frequency_penalty: f64,
    pub max_tokens: u64,
    pub presence_penalty: f64,
    pub stop_sequences: Vec<String>,
    pub temperature: f64,
    pub top_k: u64,
    pub top_p: u64,
}


impl ChatOptions for DefaultChatOptions {
    fn get_model(&self) -> &str {
        &self.model
    }

    fn get_frequency_penalty(&self) -> f64 {
        self.frequency_penalty
    }

    fn get_max_tokens(&self) -> u64 {
        self.max_tokens
    }

    fn get_presence_penalty(&self) -> f64 {
        self.presence_penalty
    }

    fn get_stop_sequences(&self) -> &Vec<String> {
        &self.stop_sequences
    }

    fn get_temperature(&self) -> f64 {
        self.temperature
    }

    fn get_top_k(&self) -> u64 {
        self.top_k
    }

    fn get_top_p(&self) -> u64 {
       self.top_p
    }

    fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }

    fn set_frequency_penalty(&mut self, frequency_penalty: f64) {
        self.frequency_penalty = frequency_penalty;
    }

    fn set_max_tokens(&mut self, max_tokens: u64) {
        self.max_tokens = max_tokens;
    }

    fn set_presence_penalty(&mut self, presence_penalty: f64) {
        self.presence_penalty = presence_penalty;
    }

    fn set_stop_sequences(&mut self, stop_sequences: Vec<String>) {
        self.stop_sequences = stop_sequences;
    }

    fn set_temperature(&mut self, temperature: f64) {
        self.temperature = temperature;
    }

    fn set_top_k(&mut self, top_k: u64) {
       self.top_k = top_k;
    }

    fn set_top_p(&mut self, top_p: u64) {
        self.top_p = top_p;
    }
}


impl ModelOptions for DefaultChatOptions {
    
}