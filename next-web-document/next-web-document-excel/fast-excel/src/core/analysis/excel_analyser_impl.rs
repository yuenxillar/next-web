use next_web_core::error::BoxError;
use tracing::error;

use crate::core::{
    analysis::{
        excel_analyser::ExcelAnalyser, excel_read_executor::ExcelReadExecutor,
        v07::xlsx_sax_analyser::XlsxSaxAnalyser,
    },
    context::{
        analysis_context::AnalysisContext, xlsx::default_xlsx_read_context::DefaultXlsxReadContext,
    },
    error::excel_error::ExcelError,
    read::metadata::{read_sheet::ReadSheet, read_workbook::ReadWorkbook},
    support::excel_type::ExcelType,
};

pub struct ExcelAnalyserImpl<T> {
    /// The context object holding metadata and configuration for the Excel analysis process.
    analysis_context: Option<Box<dyn AnalysisContext<T>>>,
    /// The executor responsible for performing the actual analysis of the Excel file.
    excel_read_executor: Option<Box<dyn ExcelReadExecutor<T>>>,
    /// Prevent multiple shutdowns
    finished: bool,
}

impl<T> ExcelAnalyserImpl<T> {
    pub fn new(read_workbook: ReadWorkbook<T>) -> Result<Self, ExcelError> {
        let mut analyser = Self::default();

        if let Err(e) = analyser.choose_excel_executor(&read_workbook) {
            error!("Failed to choose Excel executor: {}", e.to_string());
            analyser.finish()?;

            return Err(e);
        };

        Ok(analyser)
    }

    /// Chooses the appropriate Excel execution strategy based on the type of Excel file
    /// This method deals with different types of Excel files by creating corresponding processing contexts and executors
    fn choose_excel_executor(&mut self, read_workbook: &ReadWorkbook<T>) -> Result<(), ExcelError> {
        // Determine the type of Excel file based on the provided readWorkbook
        let excel_type = ExcelType::from_read_workbook(read_workbook)?;

        match excel_type {
            ExcelType::Xls => {}
            ExcelType::Xlsx => {
                //  Directly create a context and executor for processing XLSX files
                let mut xlsx_read_context =
                    DefaultXlsxReadContext::<T>::new(read_workbook, ExcelType::Xlsx);

                self.analysis_context = Some(Box::new(xlsx_read_context));
                self.excel_read_executor = Some(Box::new(XlsxSaxAnalyser::<
                    T,
                    DefaultXlsxReadContext<T>,
                >::new()));
            }
            ExcelType::Csv => {
                // Create a context and executor for processing CSV files
            }
        };

        Ok(())
    }
}

impl<T> ExcelAnalyser<T> for ExcelAnalyserImpl<T> {
    fn analysis(
        &mut self,
        read_sheet_list: Vec<ReadSheet<T>>,
        read_all: bool,
    ) -> Result<(), BoxError> {
        todo!()
    }

    fn finish(&mut self) -> Result<(), BoxError> {
        todo!()
    }

    fn excel_executor<V: ExcelReadExecutor<T>>(&self) -> V {
        todo!()
    }

    fn analysis_context<V: AnalysisContext<T>>(&self) -> V {
        todo!()
    }
}

impl<V> Default for ExcelAnalyserImpl<V> {
    fn default() -> Self {
        Self {
            analysis_context: None,
            excel_read_executor: None,
            finished: false,
        }
    }
}
