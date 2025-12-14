use thiserror::Error;

#[derive(Error, Debug)]
pub enum PdfError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("PDF error: {0}")]
    Lopdf(#[from] lopdf::Error),
    #[error("Invalid page number: {0}")]
    InvalidPageNumber(u32),
    #[error("Custom error: {0}")]
    Custom(String),
    #[error("Operation failed: {0}")]
    Operation(String),
    #[error("Analysis error: {0}")]
    AnalysisError(String),
}
