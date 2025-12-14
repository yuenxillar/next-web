use std::fmt;
use tracing::warn;

use crate::core::poi::ss::usermodel::formula_error::FormulaError;

/// Represents a constant error code value as encoded in a constant values array.
///
/// This class is a type-safe wrapper for a 16-bit int value performing a similar job to
/// `ErrorEval`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorConstant {
    error_code: i32,
}

impl ErrorConstant {
    fn to_null() -> Self {
        Self::from_error(FormulaError::Null)
    }

    fn to_div_0() -> Self {
        Self::from_error(FormulaError::Div0)
    }

    fn to_value() -> Self {
        Self::from_error(FormulaError::Value)
    }

    fn to_ref() -> Self {
        Self::from_error(FormulaError::Ref)
    }

    fn to_name() -> Self {
        Self::from_error(FormulaError::Name)
    }

    fn to_num() -> Self {
        Self::from_error(FormulaError::Num)
    }

    fn to_na() -> Self {
        Self::from_error(FormulaError::Na)
    }

    /// Creates a new ErrorConstant with the given error code.
    /// Note: This is private, use `value_of` instead.
    pub(crate) fn new(error_code: i32) -> Self {
        Self { error_code }
    }

    /// Creates a new ErrorConstant with the given error code.
    /// Note: This is private, use `value_of` instead.
    pub(crate) fn from_error(error: FormulaError) -> Self {
        Self::new(error.get_long_code())
    }

    /// Returns the error code.
    pub fn get_error_code(&self) -> i32 {
        self.error_code
    }

    /// Returns the error text.
    pub fn get_text(&self) -> String {
        if FormulaError::is_valid_code(self.error_code) {
            return FormulaError::from_int(self.error_code).get_string();
        }
        format!("unknown error code ({})", self.error_code)
    }

    /// Creates an ErrorConstant from an error code.
    pub fn value_of(error_code: i32) -> Self {
        if FormulaError::is_valid_code(error_code) {
            match FormulaError::from_int(error_code) {
                FormulaError::Null => return Self::to_null(),
                FormulaError::Div0 => return Self::to_div_0(),
                FormulaError::Value => return Self::to_value(),
                FormulaError::Ref => return Self::to_ref(),
                FormulaError::Name => return Self::to_name(),
                FormulaError::Num => return Self::to_num(),
                FormulaError::Na => return Self::to_na(),
                _ => {}
            };
        };

        warn!("Warning - unexpected error code ({})", error_code);
        Self::new(error_code)
    }
}

impl fmt::Display for ErrorConstant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ErrorConstant [{}]", self.get_text())
    }
}
