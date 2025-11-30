use crate::core::excel_writer::ExcelWriter;

pub struct ExcelWriterSheetBuilder {}

impl ExcelWriterSheetBuilder {
    pub fn new(excel_writer: ExcelWriter) -> Self {
        ExcelWriterSheetBuilder {}
    }

    pub fn set_sheet_no(&mut self, sheet_no: u32) {}

    pub fn set_sheet_name<N: Into<Box<str>>>(&mut self, sheet_name: N) {}
}
