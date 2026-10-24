use next_web_core::DynClone;

pub trait Usage: Send + Sync
where
    Self: DynClone,
{
    fn get_prompt_tokens(&self) -> u32;

    fn get_completion_tokens(&self) -> u32;

    fn get_total_tokens(&self) -> u64 {
        (self.get_prompt_tokens() + self.get_completion_tokens()).into()
    }
}

next_web_core::clone_trait_object!(Usage);
