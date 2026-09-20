use next_web_core::{clone_trait_object, DynClone};

use crate::diagnostics::error_analysis::ErrorAnalysis;

/// A ErrorAnalyzer is used to analyze a error and provide diagnostic information that can be displayed to the user.
pub trait ErrorAnalyzer
where
    Self: DynClone,
{
    /// Returns an analysis of the given error, or `None` if no analysis
    /// was possible.
    ///
    /// The returned analysis borrows the error, so analyzers can reference
    /// the original error (and any error in its `source` chain) without taking
    /// ownership of it.
    fn analyze<'a>(
        &self,
        error: &'a (dyn std::error::Error + 'static),
    ) -> Option<ErrorAnalysis<'a>>;
}

clone_trait_object!(ErrorAnalyzer);
