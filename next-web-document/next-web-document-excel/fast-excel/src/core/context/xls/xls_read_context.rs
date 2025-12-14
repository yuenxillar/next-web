use crate::core::context::analysis_context::AnalysisContext;

pub trait XlsReadContext<T>
where
    Self: AnalysisContext<T>,
{
}
