use next_web_core::error::BoxError;

use crate::core::{
    Closeable,
    write::{
        excel_builder::ExcelBuilder, excel_builder_impl::ExcelBuilderImpl,
        metadata::write_workbook::WriteWorkbook,
    },
};

pub struct ExcelWriter<T = ExcelBuilderImpl> {
    excel_builder: T,
}

impl<T> ExcelWriter<T> where T: ExcelBuilder {

    pub fn write_
}

impl ExcelWriter {
    pub fn new(write_workbook: WriteWorkbook) -> Self {
        let excel_builder = ExcelBuilderImpl::new(write_workbook);
        Self { excel_builder }
    }
}

impl<T: ExcelBuilder> Closeable for ExcelWriter<T> {
    fn close(&mut self) -> Result<(), BoxError> {
        Ok(())
    }
}
