use crate::diagnostics::{
    base_error_analyzer::BaseErrorAnalyzerExt, error_analysis::ErrorAnalysis,
};

#[derive(Debug, Clone, Default)]
pub struct PortInUseErrorAnalyzer;

impl BaseErrorAnalyzerExt for PortInUseErrorAnalyzer {
    type Cause = std::io::Error;

    fn analyze<'a>(
        _root_error: &'a (dyn std::error::Error + 'static),
        cause: Option<&'a Self::Cause>,
    ) -> Option<ErrorAnalysis<'a>> {
        let cause = cause?;
        if cause.kind() == std::io::ErrorKind::AddrInUse {
            let description = format!(
                "Web server failed to start. Port {} was already in use.",
                cause
            );
            let action = format!(
                "Identify and stop the process that's listening on port {} or configure this application to listen on another port.",
                cause
            );
            return Some(ErrorAnalysis::new(description, Some(action), Some(cause)));
        }

        None
    }
}
