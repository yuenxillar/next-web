use std::marker::PhantomData;

use next_web_core::error::BoxError;

use crate::core::{
    analysis::{
        excel_analyser::ExcelAnalyser, excel_analyser_impl::ExcelAnalyserImpl,
        excel_read_executor::ExcelReadExecutor,
    },
    context::analysis_context::AnalysisContext,
    error::{excel_analysis_error::ExcelAnalysisError, excel_error::ExcelError},
    read::metadata::{read_sheet::ReadSheet, read_workbook::ReadWorkbook},
};

pub struct ExcelReader<T, A = ExcelAnalyserImpl<T>> {
    excel_analyser: A,

    _marker: PhantomData<T>,
}

impl<T> ExcelReader<T, ExcelAnalyserImpl<T>> {
    pub fn new(read_workbook: ReadWorkbook<T>) -> Result<Self, ExcelError> {
        let excel_analyser = ExcelAnalyserImpl::new(read_workbook)?;

        Ok(ExcelReader {
            excel_analyser,
            _marker: PhantomData,
        })
    }

    /// Parse all sheet content by default
    pub fn read_all(&mut self) -> Result<(), ExcelError> {
        self.excel_analyser
            .analysis(Vec::with_capacity(0), true)
            .map_err(|e| ExcelError::AnalysisError(ExcelAnalysisError::Custom(e.to_string())))
    }

    /// Parse the specified sheet，SheetNo start from 0
    pub fn read(&mut self, read_sheet: Vec<ReadSheet<T>>) -> Result<(), ExcelError> {
        self.excel_analyser
            .analysis(read_sheet, false)
            .map_err(|e| ExcelError::AnalysisError(ExcelAnalysisError::Custom(e.to_string())))
    }

    pub fn analysis_context(&self) -> impl AnalysisContext<T> {
        self.excel_analyser.analysis_context()
    }

    pub fn excel_executor(&self) -> impl ExcelReadExecutor<T> {
        self.excel_analyser.excel_executor()
    }

    pub fn finish(&mut self) -> Result<(), ExcelError> {
        self.excel_analyser
            .finish()
            .map_err(|e| ExcelError::AnalysisError(ExcelAnalysisError::Custom(e.to_string())))
    }

    pub fn get_results(&self) -> Vec<T> {
        Vec::default()
    }
}
