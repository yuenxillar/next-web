use crate::core::cache::read_cache::ReadCache;
use crate::core::cache::selector::read_cache_selector::ReadCacheSelector;
use crate::core::enums::cell_extra_type::CellExtraType;
use crate::core::enums::read_default_return::ReadDefaultReturn;
use crate::core::error::excel_error::ExcelError;
use crate::core::event::sync_read_listener::SyncReadListener;
use crate::core::excel_reader::ExcelReader;
use crate::core::metadata::parameter_builder::ParameterBuilder;
use crate::core::read::builder::base_excel_reader_parameter_builder::BaseExcelReaderParameterBuilder;
use crate::core::read::builder::excel_reader_sheet_builder::ExcelReaderSheetBuilder;
use crate::core::read::metadata::read_workbook::ReadWorkbook;
use crate::core::support::excel_type::ExcelType;

use std::collections::HashSet;
use std::path::Path;

#[cfg(feature = "async")]
use tokio::fs::File;
#[cfg(feature = "async")]
use tokio::io;

#[cfg(not(feature = "async"))]
use std::fs::File;
#[cfg(not(feature = "async"))]
use std::io;

/// ExcelReaderBuilder for constructing ExcelReader instances
pub struct ExcelReaderBuilder<T> {
    read_workbook: ReadWorkbook<T>,
}

impl<T> ExcelReaderBuilder<T> {
    /// Creates a new ExcelReaderBuilder with default configuration
    pub fn new() -> Self {
        ExcelReaderBuilder {
            read_workbook: ReadWorkbook::<T>::default(),
        }
    }

    /// Sets the Excel file type (xls, xlsx, csv)
    pub fn excel_type(mut self, excel_type: ExcelType) -> Self {
        self.read_workbook.set_excel_type(excel_type);
        self
    }

    /// Sets the input stream to read from
    #[cfg(not(feature = "async"))]
    pub fn reader<R: io::Read + 'static>(mut self, read_stream: R) -> Self {
        self.read_workbook.set_read_stream(Box::new(read_stream));
        self
    }

    /// Sets the input stream to read from
    #[cfg(feature = "async")]
    pub fn reader<R: io::AsyncRead + 'static>(mut self, read_stream: R) -> Self {
        self.read_workbook.set_reader(Box::new(read_stream));
        self
    }

    /// Sets the file to read from
    ///
    /// If both 'input_stream' and 'file' are provided, file takes precedence
    pub fn file(mut self, file: File) -> Self {
        self.read_workbook.set_file(file);
        self
    }

    /// Sets the file to read from using a String path
    ///
    /// If both 'input_stream' and 'file' are provided, file takes precedence
    #[cfg(not(feature = "async"))]
    pub fn file_with_path_name<S: AsRef<Path>>(mut self, path_name: S) -> io::Result<Self> {
        self.read_workbook.set_file(File::open(path_name)?);
        self
    }

    /// Sets the file to read from using a String path
    ///
    /// If both 'input_stream' and 'file' are provided, file takes precedence
    #[cfg(feature = "async")]
    pub async fn file_with_path_name<S: AsRef<Path>>(
        mut self,
        path_name: S,
    ) -> Result<ExcelReaderBuilder<T>, ExcelError> {
        self.read_workbook.set_file(
            File::open(path_name)
                .await
                .map_err(|e| ExcelError::CommonError(e.to_string()))?,
        );
        Ok(self)
    }

    /// Sets the charset for CSV files
    ///
    /// Only applies to CSV file format
    pub fn charset<S: ToString>(mut self, charset: S) -> Self {
        self.read_workbook.set_charset(charset);
        self
    }

    /// Sets whether to ignore empty rows
    ///
    /// Default is true
    pub fn ignore_empty_row(mut self, ignore: bool) -> Self {
        self.read_workbook.set_ignore_empty_row(ignore);
        self
    }

    /// Sets a custom object that can be accessed in AnalysisEventListener
    ///
    /// Accessible via AnalysisContext::get_custom() in the listener
    pub fn custom_object(mut self, custom_object: T) -> Self {
        self.read_workbook.set_custom_object(custom_object);
        self
    }

    /// Sets the cache for storing temporary data to save memory
    pub fn read_cache<R: ReadCache + 'static>(mut self, read_cache: R) -> Self {
        self.read_workbook.set_read_cache(read_cache);
        self
    }

    /// Sets the cache selector
    ///
    /// Default uses SimpleReadCacheSelector1
    pub fn read_cache_selector<S: ReadCacheSelector + 'static>(mut self, selector: S) -> Self {
        self.read_workbook.set_read_cache_selector(selector);
        self
    }

    /// Sets the password for encrypted files
    pub fn password<S: ToString>(mut self, password: S) -> Self {
        self.read_workbook.set_password(password);
        self
    }

    /// Sets the SAXParserFactory class name for reading xlsx files
    ///
    /// The default will be automatically detected
    /// Example: "com.sun.org.apache.xerces.internal.jaxp.SAXParserFactoryImpl"
    pub fn xlsx_sax_parser_factory_name<S: ToString>(mut self, factory_name: S) -> Self {
        self.read_workbook
            .set_xlsx_saxparser_factory_name(factory_name);
        self
    }

    /// Adds extra information type to read
    ///
    /// By default, no extra information is read
    pub fn extra_read(mut self, extra_type: CellExtraType) -> Self {
        if self.read_workbook.get_extra_read_set().is_none() {
            self.read_workbook.set_extra_read_set(HashSet::new());
        }
        self.read_workbook
            .get_extra_read_set()
            .unwrap()
            .insert(extra_type);

        self
    }

    /// Sets whether to use the default listener
    ///
    /// Default is true. ModelBuildEventListener is loaded by default for object conversion.
    pub fn use_default_listener(mut self, use_default: bool) -> Self {
        self.read_workbook.set_use_default_listener(use_default);
        self
    }

    /// Sets the default return type when reading without target class
    ///
    /// Only effective when use_default_listener is true or null
    pub fn read_default_return(mut self, default_return: ReadDefaultReturn) -> Self {
        self.read_workbook.set_read_default_return(default_return);
        self
    }

    /// Sets the maximum number of rows to read
    pub fn num_rows(mut self, num_rows: u32) -> Self {
        self.read_workbook.set_num_rows(num_rows);
        self
    }

    /// Builds and returns an ExcelReader instance
    pub fn build(self) -> Result<ExcelReader<T>, ExcelError> {
        ExcelReader::new(self.read_workbook)
    }

    /// Reads all data and automatically closes the reader
    pub fn do_read_all(self) -> Result<(), ExcelError> {
        let mut excel_reader = self.build()?;
        excel_reader.read_all()?;
        Ok(())
    }

    /// Creates a sheet builder for the first sheet
    pub fn sheet(self) -> Result<ExcelReaderSheetBuilder<T>, ExcelError> {
        self.sheet_with_details(None, None)
    }

    /// Creates a sheet builder for the specified sheet number
    pub fn sheet_by_number(self, sheet_no: u32) -> Result<ExcelReaderSheetBuilder<T>, ExcelError> {
        self.sheet_with_details(Some(sheet_no), None)
    }

    /// Creates a sheet builder for the specified sheet name
    pub fn sheet_by_name<S: Into<String>>(
        self,
        sheet_name: S,
    ) -> Result<ExcelReaderSheetBuilder<T>, ExcelError> {
        self.sheet_with_details(None, Some(sheet_name.into()))
    }

    /// Creates a sheet builder with both sheet number and name
    pub fn sheet_with_details(
        self,
        sheet_no: Option<u32>,
        sheet_name: Option<String>,
    ) -> Result<ExcelReaderSheetBuilder<T>, ExcelError> {
        let mut builder = ExcelReaderSheetBuilder::new(self.build()?);

        if let Some(no) = sheet_no {
            builder = builder.sheet_no(no);
        }

        if let Some(name) = sheet_name {
            builder = builder.sheet_name(name);
        }

        Ok(builder)
    }
}

impl<T> ExcelReaderBuilder<T>
where
    T: Clone + 'static,
{
    /// Synchronously reads all data and returns results as a list
    pub fn do_read_all_sync(mut self) -> Result<Vec<T>, ExcelError> {
        let sync_listener = SyncReadListener::<T>::default();
        self.register_read_listener(sync_listener);
        let mut excel_reader = self.build()?;
        excel_reader.read_all()?;
        excel_reader.finish()?;

        // Get results from sync listener
        Ok(excel_reader.get_results())
    }
}

impl<T> BaseExcelReaderParameterBuilder<T, ReadWorkbook<T>> for ExcelReaderBuilder<T> {}

impl<T> ParameterBuilder<T, ReadWorkbook<T>> for ExcelReaderBuilder<T> {
    fn parameter(&mut self) -> &mut ReadWorkbook<T> {
        &mut self.read_workbook
    }
}

impl<T> Default for ExcelReaderBuilder<T> {
    fn default() -> Self {
        Self {
            read_workbook: Default::default(),
        }
    }
}
