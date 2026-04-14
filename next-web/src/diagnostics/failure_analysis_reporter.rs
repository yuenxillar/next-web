use next_web_core::{clone_trait_object, DynClone};

use crate::diagnostics::failure_analysis::FailureAnalysis;

pub trait FailureAnalysisReporter
where
    Self: Send + Sync,
    Self: DynClone,
{
    /// Reports the given failureAnalysis to the user.
    fn report(&mut self, analysis: &FailureAnalysis);
}

clone_trait_object!(FailureAnalysisReporter where Self: Send + Sync);
