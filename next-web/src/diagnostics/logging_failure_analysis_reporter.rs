use rudi_dev::singleton;
use tracing::{enabled, Level};

use crate::diagnostics::{
    failure_analysis::FailureAnalysis, failure_analysis_reporter::FailureAnalysisReporter,
};

#[singleton(binds = [Self::into_failure_analysis_reporter])]
#[derive(Debug, Clone, Default)]
pub struct LoggingFailureAnalysisReporter;

impl LoggingFailureAnalysisReporter {
    pub fn into_failure_analysis_reporter(self) -> Box<dyn FailureAnalysisReporter> {
        Box::new(self)
    }
}

impl LoggingFailureAnalysisReporter {
    fn build_message(&self, failure_analysis: &FailureAnalysis) -> String {
        let mut msg = String::with_capacity(100);
        msg.push_str("\n\n");
        msg.push_str("***************************\n");
        msg.push_str("APPLICATION FAILED TO START\n");
        msg.push_str("***************************\n\n");
        msg.push_str("Description:\n\n");
        msg.push_str(&format!("{}\n", failure_analysis.description()));
        if let Some(action) = failure_analysis.action().filter(|s| !s.is_empty()) {
            msg.push_str("\nAction:\n\n");
            msg.push_str(&format!("{}\n", action));
        }

        msg
    }
}

impl FailureAnalysisReporter for LoggingFailureAnalysisReporter {
    fn report(&mut self, failure_analysis: &FailureAnalysis) {
        if enabled!(Level::DEBUG) {
            tracing::debug!(
                case = ?failure_analysis.cause(),
                "Application failed to start due to an error",
            );
        }

        if enabled!(Level::ERROR) {
            tracing::error!("{}", self.build_message(&failure_analysis));
        }
    }
}
