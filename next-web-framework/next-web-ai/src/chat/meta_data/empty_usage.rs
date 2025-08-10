use crate::chat::meta_data::usage::Usage;

#[derive(Clone)]
pub struct EmptyUsage;

impl Usage for EmptyUsage {
    fn get_prompt_tokens(&self) -> u64 {
        0
    }

    fn get_completion_tokens(&self) -> u64 {
        0
    }
}
