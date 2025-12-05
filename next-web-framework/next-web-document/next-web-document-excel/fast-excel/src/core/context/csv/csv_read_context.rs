use crate::core::{
    context::analysis_context::AnalysisContext,
    read::metadata::holder::csv::{
        csv_read_sheet_holder::CsvReadSheetHolder, csv_read_workbook_holder::CsvReadWorkbookHolder,
    },
};

pub trait CsvReadContext<T>
where
    Self: AnalysisContext<T>,
{
    /// All information about the workbook you are currently working on.
    fn csv_read_workbook_holder(&mut self) -> &mut CsvReadWorkbookHolder;

    /// All information about the sheet you are currently working on.
    fn csv_read_sheet_holder(&mut self) -> &mut CsvReadSheetHolder;
}
