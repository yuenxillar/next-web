use crate::core::error::excel_analysis_error::ExcelAnalysisError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExcelError {
    AnalysisError(ExcelAnalysisError),
    CommonError(String),
    DataConvertError(String),
    GenerateError(String),
}
