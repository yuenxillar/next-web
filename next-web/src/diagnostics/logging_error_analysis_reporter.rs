use tracing::{enabled, Level};

use crate::diagnostics::{
    error_analysis::ErrorAnalysis, error_analysis_reporter::ErrorAnalysisReporter,
};

#[derive(Debug, Clone, Default)]
pub struct LoggingErrorAnalysisReporter;

impl LoggingErrorAnalysisReporter {
    fn build_message(&self, error_analysis: &ErrorAnalysis) -> String {
        let mut msg = String::with_capacity(100);
        msg.push_str("\n\n");
        msg.push_str("***************************\n");
        msg.push_str("APPLICATION FAILED TO START\n");
        msg.push_str("***************************\n\n");
        msg.push_str("Description:\n\n");
        msg.push_str(&format!("{}\n", error_analysis.description()));
        if let Some(action) = error_analysis.action().filter(|s| !s.is_empty()) {
            msg.push_str("\nAction:\n\n");
            msg.push_str(&format!("{}\n", action));
        }

        msg
    }
}

impl ErrorAnalysisReporter for LoggingErrorAnalysisReporter {
    fn report(&mut self, error_analysis: &ErrorAnalysis) {
        if enabled!(Level::DEBUG) {
            tracing::debug!(
                case = ?error_analysis.cause(),
                "Application failed to start due to an error",
            );
        }

        if enabled!(Level::ERROR) {
            tracing::error!("{}", self.build_message(&error_analysis));
        }
    }
}
