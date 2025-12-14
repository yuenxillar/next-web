use crate::core::poi::ss::usermodel::formula_error::FormulaError;

/// Evaluations for formula errors
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ErrorEval {
    error: FormulaError,
}

impl ErrorEval {
    /// **#NULL!** - Intersection of two cell ranges is empty
    pub fn null_intersection() -> Self {
        ErrorEval::new(FormulaError::Null)
    }

    /// **#DIV/0!** - Division by zero
    pub fn div_zero() -> Self {
        ErrorEval::new(FormulaError::Div0)
    }

    /// **#VALUE!** - Wrong type of operand
    pub fn value_invalid() -> Self {
        ErrorEval::new(FormulaError::Value)
    }

    /// **#REF!** - Illegal or deleted cell reference
    pub fn ref_invalid() -> Self {
        ErrorEval::new(FormulaError::Ref)
    }

    /// **#NAME?** - Wrong function or range name
    pub fn name_invalid() -> Self {
        ErrorEval::new(FormulaError::Name)
    }

    /// **#NUM!** - Value range overflow
    pub fn num_error() -> Self {
        ErrorEval::new(FormulaError::Num)
    }

    /// **#N/A** - Argument or function not available
    pub fn na() -> Self {
        ErrorEval::new(FormulaError::Na)
    }

    /// POI internal error codes - Function not implemented
    pub fn function_not_implemented() -> Self {
        ErrorEval::new(FormulaError::FunctionNotImplemented)
    }

    /// Note - Excel does not seem to represent this condition with an error code
    pub fn circular_ref_error() -> Self {
        ErrorEval::new(FormulaError::CircularRef)
    }

    /// Private constructor
    fn new(error: FormulaError) -> Self {
        Self { error }
    }

    /// Translates an Excel internal error code into the corresponding POI ErrorEval instance
    ///
    /// # Arguments
    /// * `error_code` - An error code listed in `FormulaError`
    ///
    /// # Panics
    /// Panics if an unknown error_code is specified
    pub fn from_code(error_code: i32) -> Self {
        let error = FormulaError::from_code(error_code);
        Self::from_formula_error(&error)
    }

    /// Get ErrorEval from FormulaError
    fn from_formula_error(error: &FormulaError) -> Self {
        match error {
            FormulaError::Null => Self::null_intersection(),
            FormulaError::Div0 => Self::div_zero(),
            FormulaError::Value => Self::value_invalid(),
            FormulaError::Ref => Self::ref_invalid(),
            FormulaError::Name => Self::name_invalid(),
            FormulaError::Num => Self::num_error(),
            FormulaError::Na => Self::na(),
            FormulaError::FunctionNotImplemented => Self::function_not_implemented(),
            FormulaError::CircularRef => Self::circular_ref_error(),
            _ => panic!("Unhandled error type for code {}", error.get_code()),
        }
    }

    /// Converts error codes to text. Handles non-standard error codes OK.
    /// For debug/test purposes (and for formatting error messages).
    ///
    /// # Returns
    /// The String representation of the specified Excel error code.
    pub fn get_text(error_code: i32) -> String {
        if FormulaError::is_valid_code(error_code) {
            FormulaError::from_code(error_code).get_string()
        } else {
            // Give a special string, based on ~, to make clear this isn't a standard Excel error
            format!("~non~std~err({})~", error_code)
        }
    }

    /// Returns the error code
    pub fn get_error_code(&self) -> i32 {
        self.error.get_long_code()
    }

    /// Returns the error string
    pub fn get_error_string(&self) -> String {
        self.error.get_string()
    }
}

impl std::fmt::Display for ErrorEval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ErrorEval [{}]", self.get_error_string())
    }
}
