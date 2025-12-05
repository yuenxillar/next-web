use std::{error::Error, fmt::Display};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExcelAnalysisError {
    Stop(Option<String>),
    StopSheet(Option<String>),
    Custom(String),
}

impl Error for ExcelAnalysisError {}
impl Display for ExcelAnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExcelAnalysisError::Stop(msg) => {
                write!(f, "Stop error: {}", msg.as_deref().unwrap_or("unknown"))
            }
            ExcelAnalysisError::StopSheet(msg) => write!(
                f,
                "StopSheet error: {}",
                msg.as_deref().unwrap_or("unknown")
            ),
            ExcelAnalysisError::Custom(msg) => write!(f, "Custom error: {}", msg),
        }
    }
}
