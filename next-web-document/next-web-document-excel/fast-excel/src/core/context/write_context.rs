use crate::core::{
    enums::write_type::WriteType,
    error::excel_error::ExcelError,
    write::metadata::{
        holder::{
            write_holder::WriteHolder, write_sheet_holder::WriteSheetHolder,
            write_table_holder::WriteTableHolder, write_workbook_holder::WriteWorkbookHolder,
        },
        write_sheet::WriteSheet,
        write_table::WriteTable,
    },
};

pub trait WriteContext {
    fn current_sheet(
        &self,
        write_sheet: WriteSheet,
        write_type: WriteType,
    ) -> Result<(), ExcelError>;

    fn current_table(&self, write_table: WriteTable) -> Result<(), ExcelError>;

    fn write_workbook_holder(&mut self) -> &mut WriteWorkbookHolder;

    fn write_sheet_holder(&mut self) -> &mut WriteSheetHolder;

    fn write_table_holder(&mut self) -> &mut WriteTableHolder;

    fn current_write_holder(&self) -> &dyn WriteHolder;

    fn finish(&mut self, on_rror: bool);
}
