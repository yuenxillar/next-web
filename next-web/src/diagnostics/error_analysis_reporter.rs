use next_web_core::{DynClone, clone_trait_object};

use crate::diagnostics::error_analysis::ErrorAnalysis;

pub trait ErrorAnalysisReporter
where
    Self: Send + Sync,
    Self: DynClone,
{
    /// Reports the given ErrorAnalysis to the user.
    fn report(&mut self, analysis: &ErrorAnalysis);
}

clone_trait_object!(ErrorAnalysisReporter where Self: Send + Sync);
