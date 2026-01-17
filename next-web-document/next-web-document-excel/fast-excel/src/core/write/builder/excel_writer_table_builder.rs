use crate::core::error::excel_error::ExcelError;
use crate::core::excel_writer::ExcelWriter;
use crate::core::metadata::parameter_builder::ParameterBuilder;
use crate::core::write::metadata::write_sheet::WriteSheet;
use crate::core::write::metadata::write_table::WriteTable;

/// Builder for creating WriteTable configurations
pub struct ExcelWriterTableBuilder {
    /// Excel writer instance (optional, for direct write operations)
    excel_writer: Option<ExcelWriter>,

    /// Write sheet configuration
    write_sheet: Option<WriteSheet>,

    /// Write table configuration being built
    write_table: WriteTable,
}

impl ExcelWriterTableBuilder {
    /// Creates a new builder with ExcelWriter and WriteSheet
    ///
    /// Used when chaining from FastExcelFactory::write().sheet().table()
    pub fn new(excel_writer: ExcelWriter, write_sheet: WriteSheet) -> Self {
        Self {
            excel_writer: Some(excel_writer),
            write_sheet: Some(write_sheet),
            write_table: Default::default(),
        }
    }

    /// Sets the table number (starting from 0)
    ///
    /// # Arguments
    /// * `table_no` - Table number, 0-based index
    pub fn table_no(mut self, table_no: u32) -> Self {
        self.write_table.set_table_no(table_no);
        self
    }

    /// Builds and returns the WriteTable configuration
    pub fn build(self) -> WriteTable {
        self.write_table
    }

    /// Writes data using the associated ExcelWriter
    ///
    /// # Arguments
    /// * `data` - Collection of data to write
    ///
    /// # Errors
    /// Returns ExcelGenerateException if no ExcelWriter is set
    pub fn do_write<T>(self, data: &[T]) -> Result<(), ExcelError>
    where
        T: Clone,
        T: AsRef<[u8]>,
    {
        let write_sheet = self.write_sheet;
        let excel_writer = self.excel_writer;
        let write_table = self.write_table;

        match (excel_writer, write_sheet) {
            (Some(mut writer), Some(sheet)) => {
                writer.write_data_with_table(data, sheet, write_table);
                writer.finish();
                Ok(())
            }
            _ => Err(ExcelError::GenerateError(
                "Must use 'FastExcelFactory.write().sheet().table()' to call this method".into(),
            )),
        }
    }

    /// Gets a mutable reference to the ExcelWriter (if set)
    pub fn excel_writer_mut(&mut self) -> Option<&mut ExcelWriter> {
        self.excel_writer.as_mut()
    }

    /// Sets the ExcelWriter
    pub fn set_excel_writer(&mut self, excel_writer: ExcelWriter) {
        self.excel_writer = Some(excel_writer);
    }

    /// Gets a reference to the WriteSheet (if set)
    pub fn write_sheet(&self) -> Option<&WriteSheet> {
        self.write_sheet.as_ref()
    }

    /// Sets the WriteSheet
    pub fn set_write_sheet(&mut self, write_sheet: WriteSheet) {
        self.write_sheet = Some(write_sheet);
    }
}

impl ParameterBuilder<(), WriteTable> for ExcelWriterTableBuilder {
    fn parameter(&mut self) -> &mut WriteTable {
        &mut self.write_table
    }
}

impl Default for ExcelWriterTableBuilder {
    fn default() -> Self {
        Self {
            excel_writer: None,
            write_sheet: None,
            write_table: Default::default(),
        }
    }
}
