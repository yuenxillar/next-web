use next_web_core::{clone_trait_object, DynClone};

pub trait ErrorChainAnalyzer
where
    Self: DynClone,
    Self: Sync + Send,
{
    fn init_extractor_map(&mut self);
}

clone_trait_object!(ErrorChainAnalyzer);

#[derive(Clone)]
pub struct BaseErrorChainAnalyzer {}

impl BaseErrorChainAnalyzer {}

impl ErrorChainAnalyzer for BaseErrorChainAnalyzer {
    fn init_extractor_map(&mut self) {}
}

impl Default for BaseErrorChainAnalyzer {
    fn default() -> Self {
        Self {}
    }
}
