use crate::core::read::metadata::{
    holder::{
        read_holder::ReadHolder, read_row_holder::ReadRowHolder,
        read_sheet_holder::ReadSheetHolder, read_workbook_holder::ReadWorkbookHolder,
    },
    read_sheet::ReadSheet,
};

pub trait AnalysisContext {
    /// Select the current table
    fn current_sheet(&self, read_sheet: &ReadSheet);

    /// All information about the workbook you are currently working on
    fn read_workbook_holder(&mut self) -> &mut ReadWorkbookHolder;

    /// All information about the sheet you are currently working on
    fn read_sheet_holder(&mut self) -> &mut ReadSheetHolder;

    /// Set row of currently operated cell
    fn set_read_row_holder(&self, read_row_holder: &ReadRowHolder);

    /// Row of currently operated cell
    fn read_row_holder(&mut self) -> &mut ReadRowHolder;

    fn current_read_holder(&self) -> &dyn ReadHolder;
}
