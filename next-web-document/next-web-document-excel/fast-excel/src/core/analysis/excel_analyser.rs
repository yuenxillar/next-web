use next_web_core::error::BoxError;

use crate::core::{
    analysis::excel_read_executor::ExcelReadExecutor, context::analysis_context::AnalysisContext,
    read::metadata::read_sheet::ReadSheet,
};

pub trait ExcelAnalyser<T> {
    fn analysis(
        &mut self,
        read_sheet_list: Vec<ReadSheet<T>>,
        read_all: bool,
    ) -> Result<(), BoxError>;

    fn finish(&mut self) -> Result<(), BoxError>;

    fn excel_executor<V: ExcelReadExecutor<T>>(&self) -> V;

    fn analysis_context<V: AnalysisContext<T>>(&self) -> V;
}
