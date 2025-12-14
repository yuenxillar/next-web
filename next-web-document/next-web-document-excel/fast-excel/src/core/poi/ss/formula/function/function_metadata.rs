use std::fmt;

/// Holds information about Excel built-in functions.
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    index: i32,
    name: String,
    min_params: u16,
    max_params: u16,
    return_class_code: u8,
    parameter_class_codes: Vec<u8>,
}

impl FunctionMetadata {
    /// max_params=30 in functionMetadata.txt means the maximum number arguments supported
    /// by the given version of Excel. Validation routines should take the actual limit (Excel 97 or 2007)
    /// from the SpreadsheetVersion enum.
    /// Perhaps a value like 'M' should be used instead of '30' in functionMetadata.txt
    /// to make that file more version neutral.
    const FUNCTION_MAX_PARAMS: u16 = 30;

    /// Create new FunctionMetadata
    pub fn new(
        index: i32,
        name: String,
        min_params: u16,
        max_params: u16,
        return_class_code: u8,
        parameter_class_codes: Vec<u8>,
    ) -> Self {
        FunctionMetadata {
            index,
            name,
            min_params,
            max_params,
            return_class_code,
            parameter_class_codes,
        }
    }

    /// Get the function index
    pub fn get_index(&self) -> i32 {
        self.index
    }

    /// Get the function name
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Get the minimum number of parameters
    pub fn get_min_params(&self) -> u16 {
        self.min_params
    }

    /// Get the maximum number of parameters
    pub fn get_max_params(&self) -> u16 {
        self.max_params
    }

    /// Check if the function has fixed number of arguments
    pub fn has_fixed_args_length(&self) -> bool {
        self.min_params == self.max_params
    }

    /// Get the return class code
    pub fn get_return_class_code(&self) -> u8 {
        self.return_class_code
    }

    /// Get a copy of parameter class codes
    pub fn get_parameter_class_codes(&self) -> Vec<u8> {
        self.parameter_class_codes.clone()
    }

    /// Some varargs functions (like VLOOKUP) have a specific limit to the number of arguments that
    /// can be passed. Other functions (like SUM) don't have such a limit. For those functions,
    /// the spreadsheet version determines the maximum number of arguments that can be passed.
    ///
    /// # Returns
    /// * `true` if this function can take the maximum number of arguments allowable by the SpreadsheetVersion
    pub fn has_unlimited_varargs(&self) -> bool {
        self.max_params == Self::FUNCTION_MAX_PARAMS
    }
}

impl fmt::Display for FunctionMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FunctionMetadata [{} {}]", self.index, self.name)
    }
}
