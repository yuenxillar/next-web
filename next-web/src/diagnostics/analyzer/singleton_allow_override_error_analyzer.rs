use crate::diagnostics::{error_analysis::ErrorAnalysis, error_analyzer::ErrorAnalyzer};

#[derive(Debug, Clone, Default)]
pub struct SingletonDefinitionOverrideErrorAnalyzer;

impl ErrorAnalyzer for SingletonDefinitionOverrideErrorAnalyzer {
    fn analyze<'a>(
        &self,
        _error: &'a (dyn std::error::Error + 'static),
    ) -> Option<ErrorAnalysis<'a>> {
        let action = "Consider renaming one of the singleton or enabling \
        overriding by setting next.appliation.context.allow_override=true";

        let error_analysis =
            ErrorAnalysis::new("Singleton definition override error", Some(action), None);

        Some(error_analysis)
    }
}
