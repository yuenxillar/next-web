use crate::{
    diagnostics::{
        analyzer::*, error_analysis::ErrorAnalysis, error_analysis_reporter::ErrorAnalysisReporter,
        error_analyzer::ErrorAnalyzer,
        logging_error_analysis_reporter::LoggingErrorAnalysisReporter,
    },
    NextWebErrorReporter,
};

/// Aggregates a set of [`ErrorAnalyzer`]s and [`ErrorAnalysisReporter`]s
/// and drives them as a single [`NextWebErrorReporter`].
///
/// When an error is reported, every registered analyzer is asked to produce a
/// [`ErrorAnalysis`] for it. The first analyzer that returns `Some` wins and
/// its analysis is handed to the registered reporters. If no analyzer matches,
/// nothing is reported and the caller is expected to fall back to its default
/// reporting behavior.
#[derive(Clone)]
pub struct ErrorAnalyzers {
    /// Analyzers consulted, in order, when an error is reported.
    analyzers: Vec<Box<dyn ErrorAnalyzer>>,
    /// Reporters invoked for a produced [`ErrorAnalysis`].
    reporters: Vec<Box<dyn ErrorAnalysisReporter>>,
}

impl ErrorAnalyzers {
    /// Creates a new `ErrorAnalyzers` instance with the given analyzers.
    ///
    /// A default [`LoggingErrorAnalysisReporter`] is installed as the sole
    /// reporter. Additional reporters can be added later.
    ///
    /// # Arguments
    ///
    /// * `analyzers` - The analyzers to consult, in order, when an error is
    ///   reported.
    ///
    /// # Returns
    ///
    /// A new [`ErrorAnalyzers`] with the given analyzers and a default
    /// logging reporter.
    pub fn new<I>(analyzers: I) -> Self
    where
        I: IntoIterator<Item = Box<dyn ErrorAnalyzer>>,
    {
        Self {
            analyzers: analyzers.into_iter().collect(),
            reporters: vec![Box::new(LoggingErrorAnalysisReporter::default())],
        }
    }

    /// Returns a `ErrorAnalyzers` instance with the default analyzers.
    pub fn with_deault_analyzers() -> Self {
        let mut error_analyzers = Self::new([]);
        error_analyzers.add_analyzer_of::<PortInUseErrorAnalyzer>();

        error_analyzers
    }

    /// Adds an analyzer to the list of analyzers.
    ///
    /// The analyzer is appended, so it is consulted after all previously
    /// registered analyzers.
    ///
    /// # Arguments
    ///
    /// * `analyzer` - The analyzer to add.
    pub fn add_analyzer<T>(&mut self, analyzer: T)
    where
        T: ErrorAnalyzer,
        T: 'static,
    {
        self.analyzers.push(Box::new(analyzer));
    }

    /// Adds a reporter to the list of reporters.
    ///
    /// The reporter is appended, so it is invoked after all previously
    /// registered reporters.
    ///
    /// # Arguments
    ///
    /// * `reporter` - The reporter to add.
    #[allow(dead_code)]
    pub fn add_reporter<T>(&mut self, reporter: T)
    where
        T: ErrorAnalysisReporter,
        T: 'static,
    {
        self.reporters.push(Box::new(reporter));
    }

    /// Adds a default analyzer to the list of analyzers.
    ///
    /// # Arguments
    ///
    /// * `analyzer` - The analyzer to add.
    fn add_analyzer_of<T>(&mut self)
    where
        T: ErrorAnalyzer,
        T: 'static,
        T: Default,
    {
        self.add_analyzer(T::default());
    }
}

impl ErrorAnalyzers {
    /// Consults the given analyzers in order and returns the first
    /// [`ErrorAnalysis`] produced.
    ///
    /// # Arguments
    ///
    /// * `error` - The error to analyze.
    /// * `analyzers` - The analyzers to consult, in order.
    ///
    /// # Returns
    ///
    /// The first analysis returned by an analyzer, or `None` if no analyzer
    /// could analyze the error.
    fn analyze<'a>(
        &self,
        error: &'a (dyn std::error::Error + 'static),
        analyzers: &[Box<dyn ErrorAnalyzer>],
    ) -> Option<ErrorAnalysis<'a>> {
        for analyzer in analyzers {
            if let Some(analysis) = analyzer.analyze(error) {
                return Some(analysis);
            }
        }

        None
    }

    /// Reports the given analysis through all registered reporters.
    ///
    /// # Arguments
    ///
    /// * `analysis` - The analysis to report, if any.
    ///
    /// # Returns
    ///
    /// `true` if an analysis was produced and reported, or `false` if there
    /// was no analysis to report or no reporter was registered.
    fn report(&mut self, analysis: Option<ErrorAnalysis>) -> bool {
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

impl NextWebErrorReporter for ErrorAnalyzers {
    /// Analyzes the given error and reports the resulting analysis, if any.
    ///
    /// # Arguments
    ///
    /// * `error` - The error to analyze and report.
    ///
    /// # Returns
    ///
    /// `true` if an analysis was produced and reported, or `false` if no
    /// analyzer matched (in which case default reporting should occur).
    fn report_error(&mut self, error: &(dyn std::error::Error + 'static)) -> bool {
        let analysis = self.analyze(error, &self.analyzers);
        self.report(analysis)
    }
}
