use std::path::Path;

#[cfg(feature = "async")]
use tokio::fs::File;

#[cfg(not(feature = "async"))]
use std::fs::File;

use crate::core::{
    metadata::parameter_builder::ParameterBuilder,
    support::excel_type::ExcelType,
    write::{
        builder::excel_writer_parameter_builder::ExcelWriterParameterBuilder,
        metadata::write_workbook::WriteWorkbook,
    },
};

pub struct ExcelWriterBuilder {
    write_workbook: WriteWorkbook,
}

impl ExcelWriterBuilder {
    pub fn password<P>(&mut self, password: P) -> &mut Self
    where
        P: Into<Box<str>>,
    {
        self.write_workbook.set_password(password);
        self
    }

    pub fn in_memory(&mut self, in_memory: bool) -> &mut Self {
        self.write_workbook.set_in_memory(in_memory);
        self
    }

    pub fn write_excel_on_error(&mut self, error: bool) -> &mut Self {
        self.write_workbook.set_write_excel_on_error(error);
        self
    }

    pub fn excel_type(&mut self, excel_type: ExcelType) -> &mut Self {
        self.write_workbook.set_excel_type(excel_type);
        self
    }

    pub fn with_bom(&mut self, with_bom: bool) -> &mut Self {
        self.write_workbook.set_with_bom(with_bom);
        self
    }

    pub async fn with_template_file(&mut self, template_file: File) -> &mut Self {
        self.write_workbook.set_template_file(template_file);
        self
    }

    pub async fn with_template_path_name<P>(&mut self, path_name: P) -> &mut Self
    where
        P: AsRef<Path>,
    {
        todo!();
    }
}

impl ExcelWriterParameterBuilder<WriteWorkbook> for ExcelWriterBuilder {}

impl ParameterBuilder<WriteWorkbook> for ExcelWriterBuilder {
    fn parameter(&mut self) -> &mut WriteWorkbook {
        &mut self.write_workbook
    }
}

impl Default for ExcelWriterBuilder {
    fn default() -> Self {
        Self {
            // Initialize fields
            write_workbook: Default::default(),
        }
    }
}
