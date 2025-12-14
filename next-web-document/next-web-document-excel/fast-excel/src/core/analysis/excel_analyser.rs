use next_web_core::error::BoxError;

use crate::{
    TodoEnum,
    core::{
        analysis::excel_read_executor::ExcelReadExecutor,
        context::analysis_context::AnalysisContext, read::metadata::read_sheet::ReadSheet,
    },
};

pub trait ExcelAnalyser {
    fn analysis(&mut self, read_sheet_list: Vec<ReadSheet>, read_all: bool)
    -> Result<(), BoxError>;

    fn finish(&mut self) -> Result<(), BoxError>;

    fn excel_executor<T: ExcelReadExecutor>(&self) -> T;

    fn analysis_context<T: AnalysisContext<TodoEnum>>(&self) -> T;
}
