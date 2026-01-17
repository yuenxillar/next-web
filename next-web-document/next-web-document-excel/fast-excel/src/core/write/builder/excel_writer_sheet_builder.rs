use std::ops::{Deref, DerefMut};

use crate::core::error::excel_error::ExcelError;
use crate::core::excel_writer::ExcelWriter;
use crate::core::write::builder::excel_writer_table_builder::ExcelWriterTableBuilder;
use crate::core::write::metadata::fill::fill_config::FillConfig;
use crate::core::write::metadata::write_sheet::WriteSheet;

/// Builder for creating WriteSheet configurations
pub struct ExcelWriterSheetBuilder {
    /// Excel writer instance (optional, for direct write operations)
    excel_writer: Option<ExcelWriter>,

    /// Write sheet configuration being built
    write_sheet: WriteSheet,
}

impl ExcelWriterSheetBuilder {
    /// Creates a new builder with ExcelWriter context
    ///
    /// Used when chaining from FastExcelFactory::write().sheet()
    pub fn new(excel_writer: ExcelWriter) -> Self {
        Self {
            excel_writer: Some(excel_writer),
            write_sheet: Default::default(),
        }
    }

    /// Sets the sheet number (starting from 0)
    ///
    /// # Arguments
    /// * `sheet_no` - Sheet number, 0-based index
    pub fn sheet_no(mut self, sheet_no: u32) -> Self {
        self.write_sheet.set_sheet_no(sheet_no);
        self
    }

    /// Sets the sheet name
    ///
    /// # Arguments
    /// * `sheet_name` - Name of the sheet
    pub fn sheet_name<S: Into<String>>(mut self, sheet_name: S) -> Self {
        self.write_sheet.set_sheet_name(sheet_name.into());
        self
    }

    /// Builds and returns the WriteSheet configuration
    pub fn build(self) -> WriteSheet {
        self.write_sheet
    }

    /// Writes data using the associated ExcelWriter
    ///
    /// # Arguments
    /// * `data` - Collection of data to write
    ///
    /// # Errors
    /// Returns ExcelError if no ExcelWriter is set
    pub fn do_write<T>(self, data: &[T]) -> Result<(), ExcelError>
    where
        T: Clone + 'static,
        T: AsRef<[u8]>,
    {
        let writer = self.excel_writer;
        match writer {
            Some(mut writer) => {
                let sheet = self.write_sheet;
                writer.write_data(data, sheet);
                writer.finish();
                Ok(())
            }
            None => Err(ExcelError::GenerateError(
                "Must use 'FastExcelFactory.write().sheet()' to call this method".into(),
            )),
        }
    }

    /// Fills data into a template using the associated ExcelWriter
    ///
    /// # Arguments
    /// * `data` - Data to fill into the template
    ///
    /// # Errors
    /// Returns ExcelError if no ExcelWriter is set
    pub fn do_fill<T>(self, data: T) -> Result<(), ExcelError>
    where
        T: AsRef<[u8]>,
    {
        self.do_fill_with_config(data, None)
    }

    /// Fills data into a template with configuration using the associated ExcelWriter
    ///
    /// # Arguments
    /// * `data` - Data to fill into the template
    /// * `fill_config` - Optional fill configuration
    ///
    /// # Errors
    /// Returns ExcelError if no ExcelWriter is set
    pub fn do_fill_with_config<T>(
        self,
        data: T,
        fill_config: Option<FillConfig>,
    ) -> Result<(), ExcelError>
    where
        T: AsRef<[u8]>,
    {
        let excel_writer = self.excel_writer;
        match excel_writer {
            Some(mut writer) => {
                let sheet = self.write_sheet;
                if let Some(config) = fill_config {
                    writer.fill_with_config(data, config, sheet);
                } else {
                    writer.fill(data, sheet);
                }

                writer.finish();
                Ok(())
            }
            None => Err(ExcelError::GenerateError(
                "Must use 'FastExcelFactory.write().sheet()' to call this method".into(),
            )),
        }
    }

    /// Creates a table builder for the current sheet
    ///
    /// Returns a table builder that can be used to configure and write tables
    pub fn table(self) -> ExcelWriterTableBuilder {
        let ExcelWriterSheetBuilder {
            excel_writer,
            write_sheet,
        } = self;

        let mut builder = ExcelWriterTableBuilder::default();
        if let Some(writer) = excel_writer {
            builder.set_excel_writer(writer);
        }
        builder.set_write_sheet(write_sheet);

        builder
    }

    /// Creates a table builder with a specific table number
    ///
    /// # Arguments
    /// * `table_no` -  table number
    pub fn table_with_number(self, table_no: u32) -> ExcelWriterTableBuilder {
        let ExcelWriterSheetBuilder {
            excel_writer,
            write_sheet,
        } = self;

        let mut builder = ExcelWriterTableBuilder::default().table_no(table_no);

        if let Some(writer) = excel_writer {
            builder.set_excel_writer(writer);
        }
        builder.set_write_sheet(write_sheet);

        builder
    }

    /// Sets the ExcelWriter
    pub fn set_excel_writer(&mut self, excel_writer: ExcelWriter) {
        self.excel_writer = Some(excel_writer);
    }

    /// Gets a reference to the WriteSheet
    pub fn write_sheet(&self) -> &WriteSheet {
        &self.write_sheet
    }
}

impl Deref for ExcelWriterSheetBuilder {
    type Target = WriteSheet;

    fn deref(&self) -> &Self::Target {
        &self.write_sheet
    }
}

impl DerefMut for ExcelWriterSheetBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.write_sheet
    }
}

impl Default for ExcelWriterSheetBuilder {
    fn default() -> Self {
        Self {
            excel_writer: None,
            write_sheet: Default::default(),
        }
    }
}
