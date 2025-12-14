use std::path::Path;

#[cfg(feature = "async")]
use tokio::fs::File;

#[cfg(not(feature = "async"))]
use std::fs::File;

use crate::core::{
    excel_writer::ExcelWriter,
    metadata::parameter_builder::ParameterBuilder,
    support::excel_type::ExcelType,
    write::{
        builder::{
            excel_writer_parameter_builder::ExcelWriterParameterBuilder,
            excel_writer_sheet_builder::ExcelWriterSheetBuilder,
        },
        metadata::write_workbook::WriteWorkbook,
    },
};
use tracing::error;

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

    pub fn file(&mut self, file: File) -> &mut Self {
        self.write_workbook.set_file(file);
        self
    }

    #[cfg(feature = "async")]
    pub async fn file_with_path<P>(&mut self, output_path_name: P) -> &mut Self
    where
        P: AsRef<Path>,
    {
        match File::open(output_path_name)
            .await
            .map_err(|e| {
                error!("Tokio open file error: {}", e);
                e
            })
            .ok()
        {
            Some(file) => self.file(file),
            None => self,
        }
    }

    #[cfg(not(feature = "async"))]
    pub fn file_with_path<P>(&mut self, output_path_name: P) -> &mut Self
    where
        P: AsRef<Path>,
    {
        match File::open(output_path_name)
            .map_err(|e| {
                error!("Std open file error: {}", e);
                e
            })
            .ok()
        {
            Some(file) => self.file(file),
            None => self,
        }
    }

    pub fn with_template_file(&mut self, template_file: File) -> &mut Self {
        self.write_workbook.set_template_file(template_file);
        self
    }

    #[cfg(feature = "async")]
    pub async fn with_template_path<P>(&mut self, path_name: P) -> &mut Self
    where
        P: AsRef<Path>,
    {
        match File::open(path_name)
            .await
            .map_err(|e| {
                error!("Tokio open file error: {}", e);
                e
            })
            .ok()
        {
            Some(file) => self.with_template_file(file),
            None => self,
        }
    }

    #[cfg(not(feature = "async"))]
    pub fn with_template_path<P>(&mut self, path_name: P) -> &mut Self
    where
        P: AsRef<Path>,
    {
        match File::open(path_name)
            .map_err(|e| {
                error!("Std open file error: {}", e);
                e
            })
            .ok()
        {
            Some(file) => self.with_template_file(file),
            None => self,
        }
    }

    pub fn build(self) -> ExcelWriter {
        ExcelWriter::new(self.write_workbook)
    }

    pub fn sheet(self) -> ExcelWriterSheetBuilder {
        self._sheet_with_no_and_name(None, None)
    }

    pub fn sheet_with_no(self, sheet_no: u32) -> ExcelWriterSheetBuilder {
        self._sheet_with_no_and_name(Some(sheet_no), None)
    }

    pub fn sheet_with_name(self, sheet_name: &str) -> ExcelWriterSheetBuilder {
        self._sheet_with_no_and_name(None, Some(sheet_name))
    }

    pub fn sheet_with_no_and_name(
        self,
        sheet_no: u32,
        sheet_name: &str,
    ) -> ExcelWriterSheetBuilder {
        self._sheet_with_no_and_name(Some(sheet_no), Some(sheet_name))
    }

    fn _sheet_with_no_and_name(
        self,
        sheet_no: Option<u32>,
        sheet_name: Option<&str>,
    ) -> ExcelWriterSheetBuilder {
        let excel_writer = self.build();

        let mut excel_writer_sheet_builder = ExcelWriterSheetBuilder::new(excel_writer);

        if let Some(sheet_no) = sheet_no {
            excel_writer_sheet_builder.set_sheet_no(sheet_no);
        }

        if let Some(sheet_name) = sheet_name {
            excel_writer_sheet_builder.set_sheet_name(sheet_name);
        }

        excel_writer_sheet_builder
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
