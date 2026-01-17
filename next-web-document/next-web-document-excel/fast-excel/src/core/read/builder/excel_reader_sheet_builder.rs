use crate::core::{
    error::excel_error::ExcelError,
    event::sync_read_listener::SyncReadListener,
    excel_reader::ExcelReader,
    metadata::parameter_builder::ParameterBuilder,
    read::{
        builder::base_excel_reader_parameter_builder::BaseExcelReaderParameterBuilder,
        metadata::read_sheet::ReadSheet,
    },
};

/// Builder for constructing a sheet reader
pub struct ExcelReaderSheetBuilder<T> {
    excel_reader: Option<ExcelReader<T>>,
    read_sheet: ReadSheet<T>,
}

impl<T> ExcelReaderSheetBuilder<T> {
    /// Creates a new ExcelReaderSheetBuilder with default settings
    pub fn new(excel_reader: ExcelReader<T>) -> Self {
        Self {
            excel_reader: Some(excel_reader),
            read_sheet: ReadSheet::<T>::default(),
        }
    }

    /// Sets the sheet number (starting from 0)
    pub fn sheet_no(mut self, sheet_no: u32) -> Self {
        self.read_sheet.set_sheet_no(sheet_no);
        self
    }

    /// Sets the sheet name
    pub fn sheet_name(mut self, sheet_name: String) -> Self {
        self.read_sheet.set_sheet_name(sheet_name);
        self
    }

    /// Sets the number of rows
    pub fn num_rows(mut self, num_rows: u32) -> Self {
        self.read_sheet.set_num_rows(num_rows);
        self
    }

    /// Builds and returns the ReadSheet
    pub fn build(self) -> ReadSheet<T> {
        self.read_sheet
    }
}

impl<T> ExcelReaderSheetBuilder<T>
where
    T: Clone + 'static,
{
    /// Performs SAX reading
    pub fn do_read(self) -> Result<(), ExcelError> {
        match self.excel_reader {
            Some(mut excel_reader) => {
                excel_reader.read(vec![self.read_sheet])?;
                excel_reader.finish()?;
            }
            None => {
                return Err(ExcelError::GenerateError(
                    "Must use 'FastExcelFactory::read().sheet()' to call this method".into(),
                ));
            }
        };

        Ok(())
    }

    /// Performs synchronous reading and returns results
    pub fn do_read_sync(mut self) -> Result<Vec<T>, ExcelError> {
        if self.excel_reader.is_none() {
            return Err(ExcelError::GenerateError(
                "Must use 'FastExcelFactory::read().sheet()' to call this method".into(),
            ));
        }

        let sync_read_listener = SyncReadListener::<T>::default();
        self.register_read_listener(sync_read_listener);

        let mut excel_reader = self.excel_reader.take().unwrap();

        excel_reader.read(vec![self.build()])?;
        excel_reader.finish()?;

        // This cast might need adjustment depending on the actual types
        Ok(excel_reader.get_results())
    }
}

impl<T> BaseExcelReaderParameterBuilder<T, ReadSheet<T>> for ExcelReaderSheetBuilder<T> {}

impl<T> ParameterBuilder<T, ReadSheet<T>> for ExcelReaderSheetBuilder<T> {
    fn parameter(&mut self) -> &mut ReadSheet<T> {
        &mut self.read_sheet
    }
}

impl<T> Default for ExcelReaderSheetBuilder<T> {
    fn default() -> Self {
        Self {
            excel_reader: None,
            read_sheet: ReadSheet::<T>::default(),
        }
    }
}
