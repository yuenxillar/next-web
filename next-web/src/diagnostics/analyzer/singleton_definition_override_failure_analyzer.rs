use rudi_dev::singleton;

use crate::diagnostics::{failure_analysis::FailureAnalysis, failure_analyzer::FailureAnalyzer};

#[singleton(binds = [Self::into_failure_analyzer])]
#[derive(Clone)]
pub struct SingletonDefinitionOverrideFailureAnalyzer;

impl FailureAnalyzer for SingletonDefinitionOverrideFailureAnalyzer {
    fn analyze(&self, _failure: &(dyn std::error::Error + 'static)) -> Option<FailureAnalysis> {
        let action = "Consider renaming one of the singleton or enabling \
        overriding by setting next.appliation.context.allow_override=true";

        let failure_analysis =
            FailureAnalysis::new("Singleton definition override failure", Some(action), None);

        Some(failure_analysis)
    }
}

impl SingletonDefinitionOverrideFailureAnalyzer {
    pub fn into_failure_analyzer(self) -> Box<dyn FailureAnalyzer> {
        Box::new(self)
    }
}
