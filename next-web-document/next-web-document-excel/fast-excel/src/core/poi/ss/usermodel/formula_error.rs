/// Enumerates error values in SpreadsheetML formula calculations.
///
/// See also OOO's excelfileformat.pdf (2.5.6)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormulaError {
    /// Internal use only - no error
    NoError = -1,
    /// **#NULL!** - Intended to indicate when two areas are required to intersect, but do not.
    ///
    /// # Example
    /// In the case of SUM(B1 C1), the space between B1 and C1 is treated as the binary
    /// intersection operator, when a comma was intended.
    Null = 0x00,
    /// **#DIV/0!** - Intended to indicate when any number, including zero, is divided by zero.
    ///
    /// # Note
    /// However, any error code divided by zero results in that error code.
    Div0 = 0x07,
    /// **#VALUE!** - Intended to indicate when an incompatible type argument is passed to a function, or
    /// an incompatible type operand is used with an operator.
    ///
    /// # Example
    /// In the case of a function argument, text was expected, but a number was provided
    Value = 0x0F,
    /// **#REF!** - Intended to indicate when a cell reference is invalid.
    ///
    /// # Example
    /// If a formula contains a reference to a cell, and then the row or column containing that cell is deleted,
    /// a #REF! error results. If a worksheet does not support 20,001 columns,
    /// OFFSET(A1,0,20000) will result in a #REF! error.
    Ref = 0x17,
    /// **#NAME?** - Intended to indicate when what looks like a name is used, but no such name has been defined.
    ///
    /// # Examples
    /// * XYZ/3, where XYZ is not a defined name.
    /// * `Total is & A10`, where neither `Total` nor `is` is a defined name.
    ///   Presumably, `"Total is " & A10` was intended.
    /// * SUM(A1C10), where the range A1:C10 was intended.
    Name = 0x1D,
    /// **#NUM!** - Intended to indicate when an argument to a function has a compatible type, but has a
    /// value that is outside the domain over which that function is defined. (This is known as a domain error.)
    ///
    /// # Example
    /// Certain calls to ASIN, ATANH, FACT, and SQRT might result in domain errors.
    ///
    /// Intended to indicate that the result of a function cannot be represented in a value of
    /// the specified type, typically due to extreme magnitude. (This is known as a range error.)
    ///
    /// # Example: FACT(1000) might result in a range error.
    Num = 0x24,
    /// **#N/A** - Intended to indicate when a designated value is not available.
    ///
    /// # Example
    /// Some functions, such as SUMX2MY2, perform a series of operations on corresponding
    /// elements in two arrays. If those arrays do not have the same number of elements, then
    /// for some elements in the longer array, there are no corresponding elements in the
    /// shorter one; that is, one or more values in the shorter array are not available.
    ///
    /// # Note
    /// This error value can be produced by calling the function NA
    Na = 0x2A,
    /// **POI specific** - Indicates that there is a circular reference in the formula
    ///
    /// # Note
    /// It is desirable to make these (arbitrary) strings look clearly different from any other
    /// value expression that might appear in a formula. In addition these error strings should
    /// look unlike the standard Excel errors. Hence tilde ('~') was used.
    CircularRef = 0xFFFFFFC4,
    /// **POI specific** - Indicates that the function required is not implemented in POI
    FunctionNotImplemented = 0xFFFFFFE2,
}

impl FormulaError {
    /// Gets the byte code of the error
    pub fn get_code(&self) -> u8 {
        (*self as i32) as u8
    }

    /// Gets the long (internal) numeric code of the error
    pub fn get_long_code(&self) -> i32 {
        *self as i32
    }

    /// Gets the string representation of the error
    pub fn to_string(&self) -> &'static str {
        match self {
            FormulaError::NoError => "(no error)",
            FormulaError::Null => "#NULL!",
            FormulaError::Div0 => "#DIV/0!",
            FormulaError::Value => "#VALUE!",
            FormulaError::Ref => "#REF!",
            FormulaError::Name => "#NAME?",
            FormulaError::Num => "#NUM!",
            FormulaError::Na => "#N/A",
            FormulaError::CircularRef => "~CIRCULAR~REF~",
            FormulaError::FunctionNotImplemented => "~FUNCTION~NOT~IMPLEMENTED~",
        }
    }

    /// Gets the string representation (alias for to_string)
    pub fn get_string(&self) -> String {
        self.to_string().into()
    }

    /// Checks if a code is a valid error code
    pub fn is_valid_code(error_code: i32) -> bool {
        match error_code {
            -1 | 0x00 | 0x07 | 0x0F | 0x17 | 0x1D | 0x24 | 0x2A | 0xFFFFFFC4 | 0xFFFFFFE2 => true,
            _ => false,
        }
    }

    /// Converts a byte to FormulaError
    ///
    /// # Panics
    /// Panics if the byte is not a valid error type
    pub fn from_byte(type_byte: u8) -> Self {
        Self::from_int(type_byte as i32)
    }

    /// Converts an integer to FormulaError
    ///
    /// # Panics
    /// Panics if the integer is not a valid error type
    pub fn from_int(type_int: i32) -> Self {
        match type_int {
            -1 => FormulaError::NoError,
            0x00 => FormulaError::Null,
            0x07 => FormulaError::Div0,
            0x0F => FormulaError::Value,
            0x17 => FormulaError::Ref,
            0x1D => FormulaError::Name,
            0x24 => FormulaError::Num,
            0x2A => FormulaError::Na,
            0xFFFFFFC4 => FormulaError::CircularRef,
            0xFFFFFFE2 => FormulaError::FunctionNotImplemented,
            _ => {
                // Try to match as byte
                match type_int as u8 {
                    0x00 => FormulaError::Null,
                    0x07 => FormulaError::Div0,
                    0x0F => FormulaError::Value,
                    0x17 => FormulaError::Ref,
                    0x1D => FormulaError::Name,
                    0x24 => FormulaError::Num,
                    0x2A => FormulaError::Na,
                    _ => panic!("Unknown error type: {}", type_int),
                }
            }
        }
    }

    /// Alias for from_int - converts an integer to FormulaError
    pub fn from_code(code: i32) -> Self {
        Self::from_int(code)
    }

    /// Converts a string to FormulaError
    ///
    /// # Panics
    /// Panics if the string is not a valid error code
    pub fn from_string(code: &str) -> Result<Self, String> {
        match code {
            "(no error)" => Ok(FormulaError::NoError),
            "#NULL!" => Ok(FormulaError::Null),
            "#DIV/0!" => Ok(FormulaError::Div0),
            "#VALUE!" => Ok(FormulaError::Value),
            "#REF!" => Ok(FormulaError::Ref),
            "#NAME?" => Ok(FormulaError::Name),
            "#NUM!" => Ok(FormulaError::Num),
            "#N/A" => Ok(FormulaError::Na),
            "~CIRCULAR~REF~" => Ok(FormulaError::CircularRef),
            "~FUNCTION~NOT~IMPLEMENTED~" => Ok(FormulaError::FunctionNotImplemented),
            _ => Err(format!("Unknown error code: {}", code)),
        }
    }
}

impl std::fmt::Display for FormulaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

// Static lookup tables (implemented as functions for simplicity)
impl FormulaError {
    /// Gets all FormulaError variants
    pub fn values() -> &'static [Self] {
        &[
            FormulaError::NoError,
            FormulaError::Null,
            FormulaError::Div0,
            FormulaError::Value,
            FormulaError::Ref,
            FormulaError::Name,
            FormulaError::Num,
            FormulaError::Na,
            FormulaError::CircularRef,
            FormulaError::FunctionNotImplemented,
        ]
    }
}
