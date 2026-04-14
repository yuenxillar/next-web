use next_web_core::{clone_trait_object, DynClone};

use crate::diagnostics::failure_analysis::FailureAnalysis;

/// A FailureAnalyzer is used to analyze a failure and provide diagnostic information that can be displayed to the user.
pub trait FailureAnalyzer
where
    Self: Send + Sync,
    Self: DynClone,
{
    /// Returns an analysis of the given failure, or None if no analysis
    /// was possible.
    fn analyze(&self, failure: &(dyn std::error::Error + 'static)) -> Option<FailureAnalysis>;
}

clone_trait_object!(FailureAnalyzer where Self: Send + Sync);
