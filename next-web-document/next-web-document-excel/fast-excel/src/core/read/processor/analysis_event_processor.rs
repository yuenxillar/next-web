use crate::core::context::analysis_context::AnalysisContext;

pub trait AnalysisEventProcessor<T> {
    fn extra(&self, analysis_context: &mut dyn AnalysisContext<T>);

    fn end_row(&self, analysis_context: &mut dyn AnalysisContext<T>);

    fn end_sheet(&self, analysis_context: &mut dyn AnalysisContext<T>);
}
