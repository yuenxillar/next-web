use std::{error::Error, marker::PhantomData};

use crate::diagnostics::{error_analysis::ErrorAnalysis, error_analyzer::ErrorAnalyzer};

#[derive(Debug, Clone)]
pub struct BaseErrorAnalyzer<T>(PhantomData<T>);

impl<T> BaseErrorAnalyzer<T>
where
    T: Error,
{
    pub fn find_cause<'a, E>(error: &'a (dyn Error + 'static)) -> Option<&'a E>
    where
        E: Error + 'static,
    {
        let mut current: Option<&'a (dyn Error + 'static)> = Some(error);

        while let Some(err) = current {
            match err.downcast_ref::<E>() {
                Some(cause) => return Some(cause),
                None => current = err.source(),
            };
        }

        None
    }
}

impl<T1> ErrorAnalyzer for T1
where
    T1: BaseErrorAnalyzerExt,
    T1: Clone,
{
    fn analyze<'a>(
        &self,
        error: &'a (dyn std::error::Error + 'static),
    ) -> Option<ErrorAnalysis<'a>> {
        let cause = BaseErrorAnalyzer::<T1::Cause>::find_cause::<T1::Cause>(error);

        <T1 as BaseErrorAnalyzerExt>::analyze(error, cause)
    }
}

pub trait BaseErrorAnalyzerExt {
    type Cause: Error + 'static;

    /// Returns an analysis of the given root error, or `None` if no analysis
    /// was possible.
    ///
    /// The analysis borrows `root_err`, so the cause found in the error chain
    /// can be referenced directly.
    fn analyze<'a>(
        root_err: &'a (dyn Error + 'static),
        cause: Option<&'a Self::Cause>,
    ) -> Option<ErrorAnalysis<'a>>;
}
