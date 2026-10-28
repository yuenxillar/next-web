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
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Feature not enabled: {0}")]
    FeatureDisabled(&'static str),
    #[error("Missing runtime dependency: {0}")]
    RuntimeDependencyMissing(String),
    #[error("Render error: {0}")]
    RenderError(String),
    #[error("OCR error: {0}")]
    OcrError(String),
    #[error("Signature error: {0}")]
    SignatureError(String),
    #[error("Unsupported Word format: {0}")]
    UnsupportedWordFormat(String),
    #[error("Word parse failed: {0}")]
    WordParseFailed(String),
    #[error("Unsupported Word element: {0}")]
    WordElementUnsupported(String),
    #[error("Word conversion failed: {0}")]
    WordConversionFailed(String),
}
