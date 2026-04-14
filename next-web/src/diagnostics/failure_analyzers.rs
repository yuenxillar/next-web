use rudi_dev::singleton;

use crate::diagnostics::{
    failure_analysis::FailureAnalysis, failure_analysis_reporter::FailureAnalysisReporter,
    failure_analyzer::FailureAnalyzer,
};

#[singleton]
#[derive(Clone)]
pub struct FailureAnalyzers {
    #[autowired(vec)]
    analyzers: Vec<Box<dyn FailureAnalyzer>>,

    #[autowired(vec)]
    reporters: Vec<Box<dyn FailureAnalysisReporter>>,
}

impl FailureAnalyzers {
    #[allow(unused)]
    pub fn new<I>(analyzers: I) -> Self
    where
        I: IntoIterator<Item = Box<dyn FailureAnalyzer>>,
    {
        Self {
            analyzers: analyzers.into_iter().collect(),
            reporters: vec![],
        }
    }

    #[allow(unused)]
    pub fn add(&mut self, analyzer: Box<dyn FailureAnalyzer>) {
        self.analyzers.push(analyzer);
    }
}

impl FailureAnalyzers {
    pub fn report_error(&mut self, failure: Box<dyn std::error::Error>) {
        let analysis = self.analyze(failure, &self.analyzers);
        self.report(analysis);
        // End
    }

    fn analyze(
        &self,
        failure: Box<dyn std::error::Error>,
        analyzers: &[Box<dyn FailureAnalyzer>],
    ) -> Option<FailureAnalysis> {
        for analyzer in analyzers {
            match analyzer.analyze(failure.as_ref()) {
                Some(analysis) => return Some(analysis),
                None => continue,
            };
        }

        None
    }

    fn report(&mut self, analysis: Option<FailureAnalysis>) -> bool {
        if self.reporters.is_empty() {
            return false;
        }

        let analysis = match analysis {
            Some(analysis) => analysis,
            None => return false,
        };

        for reporter in self.reporters.iter_mut() {
            reporter.report(&analysis);
        }

        true
    }
}
