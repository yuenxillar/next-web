use crate::core::{enums::cell_data::CellType, poi::ss::formula::eval::error_eval::ErrorEval};

/// Mimics the 'data view' of a cell. This allows formula evaluator
/// to return a CellValue instead of precasting the value to String
/// or Number or boolean type.
#[derive(Debug, Clone)]
pub struct CellValue {
    cell_type: CellType,
    number_value: f64,
    boolean_value: bool,
    text_value: Option<String>,
    error_code: i32,
}

impl CellValue {
    /// Predefined CellValue for TRUE
    pub const TRUE: Self = Self {
        cell_type: CellType::Boolean,
        number_value: 0.0,
        boolean_value: true,
        text_value: None,
        error_code: 0,
    };

    /// Predefined CellValue for FALSE
    pub const FALSE: Self = Self {
        cell_type: CellType::Boolean,
        number_value: 0.0,
        boolean_value: false,
        text_value: None,
        error_code: 0,
    };

    /// Private constructor for internal use
    fn new(
        cell_type: CellType,
        number_value: f64,
        boolean_value: bool,
        text_value: Option<String>,
        error_code: i32,
    ) -> Self {
        Self {
            cell_type,
            number_value,
            boolean_value,
            text_value,
            error_code,
        }
    }

    /// Creates a new numeric CellValue
    pub fn from_number(number_value: f64) -> Self {
        Self::new(CellType::Numeric, number_value, false, None, 0)
    }

    /// Creates a new boolean CellValue
    pub fn from_boolean(boolean_value: bool) -> Self {
        if boolean_value {
            Self::TRUE
        } else {
            Self::FALSE
        }
    }

    /// Creates a new string CellValue
    pub fn from_string(string_value: String) -> Self {
        Self::new(CellType::String, 0.0, false, Some(string_value), 0)
    }

    /// Creates a new error CellValue
    pub fn from_error(error_code: i32) -> Self {
        Self::new(CellType::Error, 0.0, false, None, error_code)
    }

    /// Returns the boolean value
    pub fn boolean_value(&self) -> bool {
        self.boolean_value
    }

    /// Returns the number value
    pub fn number_value(&self) -> f64 {
        self.number_value
    }

    /// Returns the string value, if present
    pub fn string_value(&self) -> Option<&str> {
        self.text_value.as_deref()
    }

    /// Returns the cell type
    pub fn cell_type(&self) -> CellType {
        self.cell_type
    }

    /// Returns the error value
    pub fn error_value(&self) -> u8 {
        self.error_code as u8
    }

    /// Returns the error code
    pub fn error_code(&self) -> i32 {
        self.error_code
    }

    /// Formats the CellValue as a string for display
    pub fn format_as_string(&self) -> String {
        match self.cell_type {
            CellType::Numeric => self.number_value.to_string(),
            CellType::String => {
                format!("\"{}\"", self.text_value.as_deref().unwrap_or(""))
            }
            CellType::Boolean => {
                if self.boolean_value {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            CellType::Error => ErrorEval::get_text(self.error_code),
            _ => format!("<error unexpected cell type {:?}>", self.cell_type),
        }
    }
}

impl std::fmt::Display for CellValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CellValue [{}]", self.format_as_string())
    }
}
