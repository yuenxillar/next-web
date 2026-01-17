use std::{error::Error, fmt::Display};

use next_web_core::error::BoxError;

use crate::core::error::excel_analysis_error::ExcelAnalysisError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExcelError {
    AnalysisError(ExcelAnalysisError),
    CommonError(String),
    DataConvertError(String),
    GenerateError(String),
}

impl From<ExcelAnalysisError> for ExcelError {
    fn from(error: ExcelAnalysisError) -> Self {
        ExcelError::AnalysisError(error)
    }
}

impl From<BoxError> for ExcelError {
    fn from(error: BoxError) -> Self {
        ExcelError::CommonError(error.to_string())
    }
}

impl Error for ExcelError {}
impl Display for ExcelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExcelError::AnalysisError(error) => error.fmt(f),
            ExcelError::CommonError(error) => error.fmt(f),
            ExcelError::DataConvertError(error) => error.fmt(f),
            ExcelError::GenerateError(error) => error.fmt(f),
        }
    }
}
