use std::{error::Error, marker::PhantomData};

use crate::diagnostics::{failure_analysis::FailureAnalysis, failure_analyzer::FailureAnalyzer};

#[derive(Debug, Clone)]
pub struct BaseFailureAnalyzer<T>(PhantomData<T>);

impl<T> BaseFailureAnalyzer<T>
where
    T: Error,
{
    pub fn find_cause<'a, E>(failure: &'a (dyn Error + 'static)) -> Option<&'a E>
    where
        E: Error + 'static,
    {
        let mut current: Option<&'a (dyn Error + 'static)> = Some(failure);

        while let Some(err) = current {
            match err.downcast_ref::<E>() {
                Some(cause) => return Some(cause),
                None => current = err.source(),
            };
        }

        None
    }
}

impl<T1> FailureAnalyzer for T1
where
    T1: BaseFailureAnalyzerExt,
    T1: Send + Sync,
    T1: Clone,
{
    fn analyze(&self, failure: &(dyn std::error::Error + 'static)) -> Option<FailureAnalysis> {
        let cause = BaseFailureAnalyzer::<T1::Cause>::find_cause::<T1::Cause>(failure);

        <T1 as BaseFailureAnalyzerExt>::analyze(failure, cause)
    }
}

pub trait BaseFailureAnalyzerExt {
    type Cause: Error + 'static;

    // Returns an analysis of the given  rootFailure, or None if no
    // analysis was possible.
    fn analyze(
        root_failure: &(dyn Error + 'static),
        cause: Option<&Self::Cause>,
    ) -> Option<FailureAnalysis>;
}
