use crate::diagnostics::{error_analysis::ErrorAnalysis, error_analyzer::ErrorAnalyzer};

/// The default implementation of [`ErrorAnalyzer`].
///
/// This analyzer performs a minimal, best-effort analysis of an error by
/// simply converting it into an [`ErrorAnalysis`]. It does not attempt to
/// downcast the error to any concrete type, nor does it inspect the error
/// chain for a root cause.
///
/// It is intended to be used as a fallback when no more specific analyzer
/// is registered, or as a starting point for custom analyzers.
#[derive(Debug, Clone, Default)]
pub struct DefaultErrorAnalyzer;

impl ErrorAnalyzer for DefaultErrorAnalyzer {
    /// Analyzes the given error and returns an [`ErrorAnalysis`].
    ///
    /// The analysis is constructed as follows:
    ///
    /// * The error's [`Display`](std::fmt::Display) representation is used
    ///   as the primary message.
    /// * No error code is attached (`None`).
    /// * The original error is retained as the source, allowing callers to
    ///   access the underlying error via [`ErrorAnalysis::source`].
    ///
    /// # Parameters
    ///
    /// * `error` — A trait object reference to the error being analyzed.
    ///   The `'static` bound ensures the error can be stored in the
    ///   returned [`ErrorAnalysis`] without borrowing issues.
    ///
    /// # Returns
    ///
    /// Always returns `Some(...)`, since this analyzer never fails to
    /// produce a basic analysis. The lifetime `'a` ties the returned
    /// analysis to the borrow of the input error.
    fn analyze<'a>(
        &self,
        error: &'a (dyn std::error::Error + 'static),
    ) -> Option<ErrorAnalysis<'a>> {
        Some(ErrorAnalysis::new(error.to_string(), None, Some(error)))
    }
}
