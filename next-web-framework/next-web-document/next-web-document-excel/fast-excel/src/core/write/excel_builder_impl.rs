use crate::core::write::{excel_builder::ExcelBuilder, metadata::write_workbook::WriteWorkbook};

pub struct ExcelBuilderImpl {
    write_workbook: WriteWorkbook,
}

impl ExcelBuilderImpl {
    pub fn new(write_workbook: WriteWorkbook) -> Self {
        ExcelBuilderImpl { write_workbook }
    }
}
impl ExcelBuilder for ExcelBuilderImpl {}
