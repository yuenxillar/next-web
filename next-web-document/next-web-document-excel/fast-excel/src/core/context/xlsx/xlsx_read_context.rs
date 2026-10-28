use crate::core::{
    context::analysis_context::AnalysisContext,
    read::metadata::holder::xlsx::{
        xlsx_read_sheet_holder::XlsxReadSheetHolder,
        xlsx_read_workbook_holder::XlsxReadWorkbookHolder,
    },
};

pub trait XlsxReadContext<T>
where
    Self: AnalysisContext<T>,
{
    /// All information about the workbook you are currently working on.
    /// Returns:
    /// Current workbook holder

    fn xlsx_read_workbook_holder(&mut self) -> &mut XlsxReadWorkbookHolder;

    /// All information about the sheet you are currently working on.
    /// Returns:
    /// Current sheet holder
    fn xlsx_read_sheet_holder(&mut self) -> &mut XlsxReadSheetHolder;
}
